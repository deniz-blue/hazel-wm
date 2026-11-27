# hazel-wm

will it work?

- [Lua API](docs/lua-api.md)

## X applications

Hazel has no X code. Start `xwayland-satellite` from Lua and X programs reach it as ordinary Wayland clients; `test/session.lua` shows one way, including how `DISPLAY` gets to the programs it spawns.

## Drawing from Lua

Lua can draw rectangles (with a corner radius and a border) and text on an output or on a window, and measure text before it lays it out. `test/titlebars.lua` puts a title bar on floating windows and `test/bar.lua` draws a bar on every output; they are the examples.

Text is shaped and rasterized by `cosmic-text`, which finds the installed fonts through fontconfig's configuration files. See the `wm.text`, `ShapeSpec` and `Shape` sections of the [Lua API](docs/lua-api.md).

## Outputs and workspaces

Every output shows its own workspace in `test/tiling.lua`; showing a workspace another output has swaps the two. `wm.outputs:add_virtual` makes an extra output under the nested backend, which is how the outputs code is tried without a second monitor.
