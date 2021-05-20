# ufostroker

## Add path effects to open contours in a UFO file

Given a glyph with open contours:

![](open.png)

You can apply a noodle effect:

`ufostroker -i Open.ufo -o Output.ufo noodle --size 30`

![](noodle.png)

Or you can choose a glyph from the UFO with closed contours (in this case the `star` glyph) and apply a pattern-along-path effect:

`ufostroker -i Open.ufo -o Output.ufo pattern -p star --spacing 30 --stretch true`

![](pattern.png)

Use `ufostroker --help`, `ufostroker noodle --help` or `ufostroker pattern --help` to see more options. (There are many.)

## To build

* Install Rust (a nightly toolchain is required, typically with `rustup toolchain install nightly`)
* `cargo build --release`
* Find the binary in `target/release`

## Acknowledgements

All the heavy lifting is done by Matthew Blanchard and Fred Brennan's [MFEK Math](https://github.com/MFEK/math.rlib) library.

## License

This software is licensed under the Apache License, 2.0
