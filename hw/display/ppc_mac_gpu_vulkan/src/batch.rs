//! Batches: a ring of command buffers with fences, a completion thread,
//! the per-batch bump arena, and deferred destruction.

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use ash::vk;

use crate::state::*;

fn set_done(sh: &Shared, seq: u32) {
    let mut old = sh.done_seq.load(Ordering::Acquire);
    while (seq.wrapping_sub(old) as i32) > 0 {
        match sh.done_seq.compare_exchange(old, seq, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(cur) => old = cur,
        }
    }
}

/// The completion thread: waits for each submitted batch in order,
/// publishes it as done, and calls its callback.
pub fn waiter(dev: ash::Device, sh: Arc<Shared>) {
    loop {
        let (fence, seq, done, arg) = {
            let mut q = sh.q.lock().unwrap();
            while q.is_empty() {
                q = sh.wake.wait(q).unwrap();
            }
            let it = q.front().unwrap();
            (it.fence, it.seq, it.done, it.arg)
        };
        // SAFETY: the fence belongs to a batch that is not recycled until
        // worker_seq passes it, which only this thread publishes.
        unsafe {
            let _ = dev.wait_for_fences(&[fence], true, u64::MAX);
        }
        set_done(&sh, seq);
        if let Some(d) = done {
            unsafe { d(arg as *mut c_void, seq) };
        }
        let mut q = sh.q.lock().unwrap();
        q.pop_front();
        sh.worker_seq.store(seq, Ordering::Release);
        sh.idle.notify_all();
    }
}

impl Vk {
    pub fn seq_done(&self, seq: u32) -> bool {
        (self.shared.done_seq.load(Ordering::Acquire).wrapping_sub(seq) as i32) >= 0
    }

    pub fn mark_done(&self, seq: u32) {
        set_done(&self.shared, seq);
    }

    fn destroy(&self, t: Trash) {
        unsafe {
            match t {
                Trash::Image(h) => self.dev.destroy_image(h, None),
                Trash::View(h) => self.dev.destroy_image_view(h, None),
                Trash::Mem(h) => self.dev.free_memory(h, None),
                Trash::Fb(h) => self.dev.destroy_framebuffer(h, None),
            }
        }
    }

    /// Destroy something once the work that may still use it is complete:
    /// with the open batch, else the newest submitted one, else now.
    pub fn trash(&mut self, t: Trash) {
        let target = self.cur.or_else(|| {
            self.batch
                .iter()
                .enumerate()
                .filter(|(_, b)| b.state == 2)
                .max_by_key(|(_, b)| b.seq.wrapping_sub(self.seq) as i32)
                .map(|(i, _)| i)
        });
        match target {
            Some(i) => self.batch[i].trash.push(t),
            None => self.destroy(t),
        }
    }

    /// Return a completed batch's resources.
    fn recycle(&mut self, i: usize) {
        unsafe {
            let b = &mut self.batch[i];
            let _ = self.dev.reset_fences(&[b.fence]);
            let _ = self.dev.reset_command_buffer(b.cb, vk::CommandBufferResetFlags::empty());
            for &p in &b.pools {
                let _ = self.dev.reset_descriptor_pool(p, vk::DescriptorPoolResetFlags::empty());
            }
            b.pool_cur = 0;
            let chunks: Vec<Chunk> = b.chunks.drain(..).collect();
            let trash: Vec<Trash> = b.trash.drain(..).collect();
            b.state = 0;
            for mut c in chunks {
                if c.size == ARENA_CHUNK && self.free_chunks.len() < 8 {
                    c.off = 0;
                    self.free_chunks.push(c);
                } else {
                    self.dev.destroy_buffer(c.buf, None);
                    self.dev.free_memory(c.mem, None);
                }
            }
            for t in trash {
                self.destroy(t);
            }
        }
    }

    fn reap(&mut self) {
        let ws = self.shared.worker_seq.load(Ordering::Acquire);
        for i in 0..NBATCH {
            if self.batch[i].state == 2 && (ws.wrapping_sub(self.batch[i].seq) as i32) >= 0 {
                self.recycle(i);
            }
        }
    }

    pub fn full_barrier(&self, cb: vk::CommandBuffer) {
        let mb = [vk::MemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
            .build()];
        unsafe {
            self.dev.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &mb,
                &[],
                &[],
            );
        }
    }

    /// The open batch, starting one if needed.
    pub fn batch(&mut self) -> usize {
        if let Some(i) = self.cur {
            return i;
        }
        self.reap();
        let i = loop {
            if let Some(i) = (0..NBATCH).find(|&i| self.batch[i].state == 0) {
                break i;
            }
            // all in flight: wait for the waiter to finish the oldest
            {
                let mut q = self.shared.q.lock().unwrap();
                let ws = self.shared.worker_seq.load(Ordering::Acquire);
                while self.shared.worker_seq.load(Ordering::Acquire) == ws && !q.is_empty() {
                    q = self.shared.idle.wait(q).unwrap();
                }
            }
            self.reap();
        };
        self.seq = self.seq.wrapping_add(1);
        if self.seq == 0 {
            self.seq = 1;
        }
        let seq = self.seq;
        let b = &mut self.batch[i];
        b.state = 1;
        b.seq = seq;
        let cb = b.cb;
        unsafe {
            let _ = self.dev.begin_command_buffer(
                cb,
                &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            );
        }
        // ordered after everything submitted before (and host writes)
        self.full_barrier(cb);
        self.cur = Some(i);
        self.chunk = None;
        i
    }

    pub fn cb(&mut self) -> vk::CommandBuffer {
        let i = self.batch();
        self.batch[i].cb
    }

    /// Bump allocation from the batch's host-visible arena: the buffer, the
    /// offset in it, and the mapped bytes.
    pub fn arena(&mut self, len: u64) -> Option<(vk::Buffer, u64, *mut u8)> {
        let bi = self.batch();
        let len = len.div_ceil(self.align) * self.align;
        let need_new = match self.chunk {
            Some(ci) => self.batch[bi].chunks[ci].off + len > self.batch[bi].chunks[ci].size,
            None => true,
        };
        if need_new {
            let c = if len <= ARENA_CHUNK && !self.free_chunks.is_empty() {
                self.free_chunks.pop().unwrap()
            } else {
                let size = len.max(ARENA_CHUNK);
                let (buf, mem, map) = self
                    .buffer(
                        size,
                        vk::BufferUsageFlags::UNIFORM_BUFFER
                            | vk::BufferUsageFlags::STORAGE_BUFFER
                            | vk::BufferUsageFlags::TRANSFER_SRC,
                        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                        vk::MemoryPropertyFlags::empty(),
                    )
                    .ok()?;
                Chunk { buf, mem, map, size, off: 0 }
            };
            let small = len <= ARENA_CHUNK;
            self.batch[bi].chunks.push(c);
            if small {
                self.chunk = Some(self.batch[bi].chunks.len() - 1);
            }
            // an oversized chunk serves this one allocation only
            if !small {
                let ci = self.batch[bi].chunks.len() - 1;
                let c = &mut self.batch[bi].chunks[ci];
                let off = c.off;
                c.off += len;
                return Some((c.buf, off, unsafe { c.map.add(off as usize) }));
            }
        }
        let ci = self.chunk?;
        let c = &mut self.batch[bi].chunks[ci];
        let off = c.off;
        c.off += len;
        Some((c.buf, off, unsafe { c.map.add(off as usize) }))
    }

    pub fn end_pass(&mut self) {
        if self.in_pass {
            if let Some(i) = self.cur {
                unsafe { self.dev.cmd_end_render_pass(self.batch[i].cb) };
            }
            self.in_pass = false;
        }
    }

    /// Submit the open batch; `done` is called from the completion thread
    /// when it finishes.  Returns its sequence number (0 if none was open).
    pub fn commit(&mut self, done: Option<DoneFn>, arg: *mut c_void) -> u32 {
        let Some(bi) = self.cur else {
            return 0;
        };
        self.end_pass();
        // What the device marked dirty for this batch's draws is ours.
        self.harvest();
        for i in 0..MAX_IMG {
            if self.img[i].live && self.img[i].dirty {
                self.copy_to_vram(i);
            }
        }
        let cb = self.batch[bi].cb;
        let mb = [vk::MemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ)
            .build()];
        unsafe {
            self.dev.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &mb,
                &[],
                &[],
            );
            let _ = self.dev.end_command_buffer(cb);
            let cbs = [cb];
            let si = [vk::SubmitInfo::builder().command_buffers(&cbs).build()];
            if let Err(e) = self.dev.queue_submit(self.queue, &si, self.batch[bi].fence) {
                crate::state::fail(&format!("vkQueueSubmit failed ({e:?})"));
            }
        }
        let (fence, seq) = (self.batch[bi].fence, self.batch[bi].seq);
        self.batch[bi].state = 2;
        self.cur = None;
        self.chunk = None;

        let mut q = self.shared.q.lock().unwrap();
        q.push_back(WaitItem { fence, seq, done, arg: arg as usize });
        self.shared.wake.notify_one();
        seq
    }

    /// Submit what is open and wait for everything.  True if work was pending.
    pub fn flush(&mut self) -> bool {
        self.commit(None, std::ptr::null_mut());
        let newest = self
            .batch
            .iter()
            .filter(|b| b.state == 2 && !self.seq_done(b.seq))
            .max_by_key(|b| b.seq.wrapping_sub(self.seq) as i32)
            .map(|b| (b.fence, b.seq));
        let Some((fence, seq)) = newest else {
            return false;
        };
        // A barrier opens every batch, so the newest finishing means all did.
        unsafe {
            let _ = self.dev.wait_for_fences(&[fence], true, u64::MAX);
        }
        self.mark_done(seq);
        self.stat_flushes += 1;
        true
    }
}
