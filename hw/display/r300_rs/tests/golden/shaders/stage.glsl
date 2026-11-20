#version 450
#ifdef R300_VS
layout(location = 0) in vec4 pos;
layout(location = 0) out vec4 color;
void main() { gl_Position = pos * 2.0; color = pos; }
#endif
#ifdef R300_FS
layout(location = 0) in vec4 color;
layout(location = 0) out vec4 outc;
#ifdef R300_FS_Z
layout(std140, set = 0, binding = 0) uniform U { vec4 k; };
void main() { outc = color * k; gl_FragDepth = color.x; }
#else
void main() { outc = color + vec4(0.25); }
#endif
#endif
