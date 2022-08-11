# CHIP-8

This is a [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) interpreter/emulator. This project was created to mainly learn more about low level programming, and how computers work. This project was also used as an introduction to emulator development, something that I am interested in exploring further.

## Features
 - Load ROMs using a file dialog.
 - Edit settings to change background and foreground colors, as well as the speed of emulation.
 - Save and load the CHIP-8 interpreter state to/from disk.
 - Debug CHIP-8 programs with a "debug view" that allows inspecting the 
interpreter state while the program is running.

## Reflection
(todo)

## Running
Using [cargo](https://doc.rust-lang.org/cargo/)

`$ git clone <this_repo>`

`$ cd chip8`

`$ cargo run --release`

## Resources
[Cowgod's Chip-8 Technical Reference](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM) 
was a very helpful resource during the development of this interpreter.
