import mmap, os, struct
  w, h = 1024, 768          # use virtual_size
  stride = 4096             # use stride
  bpp = 32                  # use bits_per_pixel
  fb = os.open('/dev/fb0', os.O_RDWR)
  m = mmap.mmap(fb, stride * h)

  def px(x, y, r, g, b):
      m[y*stride + x*4 : y*stride + x*4 + 4] = struct.pack('<I', (r << 16) | (g << 8) | b)

  # 1. colour bars: red, green, blue, white. Channel order and byte swap show up immediately.
  for y in range(h // 4):
      for x in range(w):
          c = x * 4 // w
          px(x, y, *[(255,0,0),(0,255,0),(0,0,255),(255,255,255)][c])

  # 2. horizontal gradient, to catch pitch and wrap errors.
  for y in range(h // 4, h // 2):
      for x in range(w):
          v = x * 255 // (w - 1)
          px(x, y, v, v, v)

  # 3. a 1-pixel checkerboard, to expose tiling and scrambling.
  for y in range(h // 2, 3 * h // 4):
      for x in range(w):
          v = 255 if (x + y) & 1 else 0
          px(x, y, v, v, v)

  # 4. small rectangles at odd offsets and sizes, to test partial writes and dirty tracking.
  for i in range(8):
      for y in range(3 * h // 4 + 5, 3 * h // 4 + 5 + 3 + i):
          for x in range(7 + i * 61, 7 + i * 61 + 5 + i * 3):
              px(x, y, 255, 128, 0)