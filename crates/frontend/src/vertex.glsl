#version 300 es

precision mediump float;

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec2 in_tex_coord;

out vec2 tex_coord;

void main() {
    tex_coord = in_tex_coord;
    gl_Position = vec4(in_position.x, in_position.y, in_position.z, 1.0);
}