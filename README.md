# Github Graph

This is a simple project to show some statistics of a github project.

## How to Run

By default, if there is a `data.json` file in the root directory, the project will use it. There is an included `data.json` that shows the stats of the `gfx-rs/wgpu`, `gfx-rs/wgpu-rs`, and `gfx-rs/naga` repositories.

`cargo run --release`

## Update Data

- Add your github token to `.token` file.
- Run the program. The program will generate a `data.json` file.
