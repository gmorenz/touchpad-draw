# Touchpad Draw

A simple program to allow the drawing of pictures on a touchpad.

This software is not in any way 'good', it is a quick tool made for my own use, I'm only really publishing it because (I at least) couldn't find another tool to do the same task. It is linux only, well it might work on BSDs such as OSX, but I would be surprised. Even on linux, it might not work without adaptation if the touchpad drivers function in a slightly different way, or something like that. 

## Installation:

Install rust (nightly, we use some unstable features), and cargo, run `cargo build`, the executable will be `target/debug/touchpad-draw`, requires SDL2 to compile (and run).

## Usage:

touchpad-draw output.png

- Space: Pause drawing
- i: Continue drawing normally
- l: Draw lines between sequential taps (useful, for, e.g., drawing axises on graphs)
- 1: Color black
- 2: Color red
- 3: Color green
- 4: Color blue

Kill using your window manager (alt-f4 on most), the output will be saved after the window is closed, give it a few seconds.

## Technical Information

Since it's likely you will need to edit the soure code if you are using this, here are some basic notes.

We take touchpad events from /dev/input/event##, the number is found simply by finding a device that supports absolute events, and assuming that this is the touchpad (works on the two laptops I have tested on). Various documents (like the [manpage] for evtest) suggest that this probably shouldn't work when synaptics is enabled... but it does for me... so your millage may vary. (If it doesn't detect any events, try running `synclient TouchpadOff=1` and see if that helps?) Events are recorded on an input thread that only reads from the event device, and does basic processing, so it is possible that we are simply winning a race to get most of the events, but (if you unhide and ungrab the cursor in the sdl code) the touchpad functions normally simultaneously, so I expect something else is happening.

In `i` mode, while the finger is held down, we draw straight lines between each `Touch` reported (unless the finger has been lifted up in the mean time), to avoid getting a dashed line as your finger moves faster then the touchpad detects it.
