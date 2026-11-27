# Lua API

Notation: `Type?` is `Type` or `nil`. `a|b` is a string that is one of the listed values. Indexes are 1-based. Coordinates are logical pixels. `Window`, `Layer` and `WmOutput` compare with `==`.

## Globals

- `wm: Wm`
- `Key: table<string, Keysym>`
- `Button: table<string, MouseButton>`
- `Point(x: number, y: number) -> Point`
- `Point(point: Point | table) -> Point`
- `Rect(x: number, y: number, width: number, height: number) -> Rect`
- `Rect(table) -> Rect`
- `spawn(program: string, args: string[]?, options: ProcessOptions?) -> integer`
- `exec(program: string, args: string[]?, options: ProcessOptions?) -> ExecResult`
- `kill(pid: integer, signal: "term"|"kill"|"int"|"hup"?) -> boolean`

## Point

- `point.x: number`
- `point.y: number`
- `point:add(other: Point) -> nil`: adds in place
- `point + other -> Point`
- `point * other -> Point`: per component

## Size

- `size.width: number`
- `size.height: number`
- a negative or NaN width or height raises an error

## Rect

- `rect.x: number`
- `rect.y: number`
- `rect.width: number`
- `rect.height: number`
- a negative size, or an edge outside the 32-bit range, raises an error
- `rect:contains(point: Point) -> boolean`
- `rect:intersects(other: Rect) -> boolean`
- `rect:intersection(other: Rect) -> Rect?`
- `rect:merge(other: Rect) -> Rect`: smallest rect containing both; raises if the result is outside the 32-bit range
- `rect:inset(amount: number) -> Rect`: shrinks every side by `amount`, never below 0x0

## Wm

- `wm.name: string`
- `wm.input: WmInput`
- `wm.clipboard: WmClipboard`
- `wm.session_lock: WmSessionLock`
- `wm.windows: WmWindows`
- `wm.layers: WmLayers`
- `wm.outputs: WmOutputs`
- `wm.text: WmText`
- `wm.is_reload: boolean`
- `wm.wayland_display: string`
- `wm.is_session_locked: boolean`
- `wm.store: table`: survives reloads, holds booleans, numbers, strings and tables
- `wm:after(delay_ms: number, callback: function) -> Timer`
- `wm:every(interval_ms: number, callback: function) -> Timer`
- `wm:time() -> number`: milliseconds since the compositor started
- `wm:screenshot(path: string, output: WmOutput?) -> nil`: writes a PNG of `output`, or of the main output when it is left out. A virtual output can be shot too
- `wm:import_environment(variables: table) -> string[]`: pushes variables to systemd and dbus, returns the errors
- `wm:reload() -> nil`
- `wm:quit() -> nil`

### Wm events

- `wm:on("ready", function() end)`
- `wm:on("unload", function() end)`
- `wm:on("session_locked", function() end)`
- `wm:on("session_unlocked", function() end)`
- `wm:on("tick", function() end)`

## Timer

- `timer:cancel() -> boolean`

## Processes

- `ProcessOptions.env: table<string, string>?`
- `ProcessOptions.cwd: string?`
- `ProcessOptions.timeout: number?`: milliseconds before SIGTERM, SIGKILL a second later
- `ProcessOptions.on_stdout: fun(line: string)?`
- `ProcessOptions.on_stderr: fun(line: string)?`
- `ProcessOptions.on_exit: fun(report: ExitReport)?`
- `ProcessOptions.survives_reload: boolean?`: keep running through a reload, callbacks dropped
- `ExitReport.status: integer?`: nil when killed by a signal
- `ExitReport.signal: integer?`
- `ExitReport.timed_out: boolean`
- `ExecResult.stdout: string`
- `ExecResult.stderr: string`
- `ExecResult` also has every `ExitReport` field

## WmWindows

- `wm.windows:topmost() -> Window?`
- `wm.windows:count() -> number`
- `wm.windows:by_id(id: integer) -> Window?`
- `wm.windows:get(index: number) -> Window?`
- `wm.windows:all() -> Window[]`
- `wm.windows:under(point: Point) -> Window?`

### WmWindows events

- `wm.windows:on("new_window", function(window: Window) end)`
- `wm.windows:on("mapped", function(window: Window) end)`
- `wm.windows:on("resized", function(window: Window) end)`: a mapped window's committed size (`window:size()`) differs from the one Lua last saw, whether the client changed it or a `set_size` was answered. Once per change; not for moves, and not when the window maps, since `mapped` already has the size
- `wm.windows:on("closed", function(window: Window) end)`
- `wm.windows:on("focus", function(window: Window?, seat: Seat) end)`: a seat's keyboard focus changed; `window` is nil when it left every window (it may be on a layer or on nothing)
- `wm.windows:on("title_changed", function(window: Window) end)`
- `wm.windows:on("app_id_changed", function(window: Window) end)`
- `wm.windows:on("parent_changed", function(window: Window) end)`
- `wm.windows:on("maximize_request", function(window: Window) end)`
- `wm.windows:on("unmaximize_request", function(window: Window) end)`
- `wm.windows:on("fullscreen_request", function(window: Window, output: WmOutput?) end)`
- `wm.windows:on("unfullscreen_request", function(window: Window) end)`
- `wm.windows:on("minimize_request", function(window: Window) end)`
- `wm.windows:on("unminimize_request", function(window: Window) end)`
- `wm.windows:on("activation_request", function(window: Window, token_age_ms: integer) end)`: tokens of any age; the handler decides whether an old one counts
- `wm.windows:on("decoration_request", function(window: Window) end)`
- `wm.windows:on("move_request", function(request: WindowDragRequest) end)`
- `wm.windows:on("resize_request", function(request: WindowDragRequest) end)`
- `wm.windows:on("bell", function(window: Window?) end)`

## Window

- `window:id() -> integer`
- `window:kind() -> "window"`
- `window:title() -> string?`
- `window:app_id() -> string?`
- `window:parent() -> Window?`
- `window:output() -> WmOutput?`
- `window:mapped() -> boolean`
- `window:position() -> Point`
- `window:set_position(position: Point) -> nil`
- `window:size() -> Size`
- `window:set_size(size: Size) -> nil`
- `window:clear_size() -> nil`: drops the requested size
- `window:states() -> WindowStates`
- `window:focused_seats() -> Seat[]`: the seats whose keyboard focus is on this window, sorted by name
- `window:set_activated(active: boolean) -> nil`
- `window:visible() -> boolean`
- `window:set_visible(visible: boolean) -> nil`
- `window:raise() -> nil`
- `window:layer() -> "below"|"normal"|"above"`
- `window:set_layer(layer: "below"|"normal"|"above") -> nil`
- `window:set_border(width: integer, color: string) -> nil`: `color` is `"#rrggbb"` or `"#rrggbbaa"`; a width over 4096 is an error
- `window:clear_border() -> nil`
- `window:set_decoration(mode: "server"|"client"|"none") -> nil`
- `window:close() -> nil`
- `window:add_shape(spec: ShapeSpec) -> Shape`: drawn with the window, at positions relative to the window's top left, and moving with it. Shapes with `z` of 0 or more are in front of the window and its border, shapes below 0 are behind the window. The window's shapes go when the window does

## WindowStates

State names: `maximized`, `fullscreen`, `resizing`, `activated`, `tiled_left`, `tiled_right`, `tiled_top`, `tiled_bottom`, `suspended`.

- `states:has(name: string) -> boolean`
- `states:list() -> string[]`
- `states:set(name: string, enabled: boolean) -> nil`

## WindowDragRequest

- `request.window: Window`
- `request.pointer: Pointer`
- `request.edges: string?`: `top`, `bottom`, `left`, `right`, `top_left`, `top_right`, `bottom_left`, `bottom_right`, `none`; nil for a move

## WmLayers

- `wm.layers:count() -> number`
- `wm.layers:get(index: number) -> Layer?`
- `wm.layers:all() -> Layer[]`
- `wm.layers:namespace(name: string) -> Layer[]`
- `wm.layers:depth(depth: "background"|"bottom"|"top"|"overlay") -> Layer[]`
- `wm.layers:on_output(name: string) -> Layer[]`
- `wm.layers:under(point: Point) -> Layer?`

### WmLayers events

- `wm.layers:on("new_surface", function(layer: Layer) end)`
- `wm.layers:on("mapped", function(layer: Layer) end)`
- `wm.layers:on("changed", function(layer: Layer) end)`: a layer's anchor, exclusive zone, exclusive edge, margin, requested size, keyboard interactivity, layer depth, `position` or `size` differs from what Lua last saw (from `mapped` or the previous `changed`). One event for all of them; compare the getters yourself to see which. Fires after the layers are arranged, so `position`, `size` and `output:usable_area()` are already the new ones. A layer that another layer or an output change moved gets one too. `size` is the size of the buffer the client last committed, so a change that resizes the layer usually fires twice: once when the compositor applies it, once when the client draws at the new size
- `wm.layers:on("remove_surface", function(layer: Layer) end)`: the layer is already gone from `wm.layers` and from `output:usable_area()`. `layer:output()` still answers; `position` and `size` are 0

## Layer

- `layer:kind() -> "layer"`
- `layer:namespace() -> string`
- `layer:layer() -> "background"|"bottom"|"top"|"overlay"`
- `layer:position() -> Point`
- `layer:size() -> Size`
- `layer:output() -> WmOutput?`: the output the layer was created on, also after it is removed; nil once that output is gone
- `layer:keyboard_interactivity() -> "none"|"exclusive"|"on_demand"`
- `layer:anchor() -> { top: boolean, bottom: boolean, left: boolean, right: boolean }`
- `layer:margin() -> { top: integer, right: integer, bottom: integer, left: integer }`
- `layer:exclusive_zone() -> integer`: the protocol value: above 0 reserves that many pixels, 0 moves out of the way of reserving layers, -1 ignores them
- `layer:exclusive_edge() -> "top"|"bottom"|"left"|"right"|nil`: only what the client set with `set_exclusive_edge`; nil when it did not. Without it the zone applies to the edge the anchors imply: the single anchored edge, or the one opposite the missing edge when three are anchored; none with zero, two or four anchors
- `layer:requested_size() -> Size`: what the client asked for; 0 in a dimension means stretch between the anchors. `size()` is what the layer got after arrangement
- `layer:close() -> nil`

Placement stays in Rust: smithay arranges the layers, and `position`, `size` and `output:usable_area()` are its result. The properties above are the inputs.

## WmSessionLock

While the session is locked or locking, Rust draws only the lock surfaces on a backdrop, sends pointer and keyboard focus to the lock client and ignores `seat:focus`. Every input event still reaches Lua first; `prevent_default()` keeps one from the lock client.

- `wm.session_lock:state() -> "unlocked"|"locking"|"locked"`
- `wm.session_lock:surfaces() -> LockSurface[]`
- `wm.session_lock:set_backdrop_color(color: string) -> nil`: `#rrggbb` or `#rrggbbaa`, black until set

## LockSurface

- `surface:output() -> WmOutput`
- `surface:has_buffer() -> boolean`

## ShortcutsInhibitor

- `inhibitor:activate() -> nil`
- `inhibitor:deactivate() -> nil`
- `inhibitor:is_active() -> boolean`
- `inhibitor:window() -> Window?`

## WmClipboard

The CLIPBOARD selection of each seat, whoever owns it. Primary selection is not exposed.

- `wm.clipboard:current(seat: Seat) -> ClipboardOffer?`
- `wm.clipboard:set(seat: Seat, contents: table) -> nil`: the compositor owns that seat's clipboard with these contents, `{ [mime_type] = data }` with Lua strings as data. An empty table clears it

### WmClipboard events

- `wm.clipboard:on("changed", function(offer: ClipboardOffer?, seat: Seat) end)`: a new owner on that seat, or `nil` when its selection was cleared. Fires from the main loop, never inside a protocol request; `set` from the handler is fine. A `set` from Lua fires it too, with origin `"lua"`

## ClipboardOffer

- `offer.origin: "wayland"|"lua"`
- `offer.seat: Seat?`: nil when the seat is gone
- `offer:mime_types() -> string[]`
- `offer:is_current() -> boolean`
- `offer:read(mime_type: string, max_bytes: integer, callback: function(data: string?, err: string?)) -> nil`: always answers later, from the main loop. `err` is set when the type is not offered, the data is over `max_bytes`, or the clipboard changed before the data arrived

## WmOutputs

`pairs(wm.outputs)` yields `name, output`.

- `wm.outputs:count() -> number`
- `wm.outputs:get(index: number) -> WmOutput?`
- `wm.outputs:all() -> WmOutput[]`
- `wm.outputs:name(name: string) -> WmOutput?`
- `wm.outputs:under(point: Point) -> WmOutput[]`
- `wm.outputs:add_virtual(name: string, size: Size) -> WmOutput`: only under the nested (winit) backend; an output with no monitor behind it, placed right of the others and drawn by the same renderer. `size` is 1x1 to 16384x16384 and the name must be new. It can be shot with `wm:screenshot(path, output)`. The tty backend raises

### WmOutputs events

- `wm.outputs:on("added", function(output: WmOutput) end)`
- `wm.outputs:on("changed", function(output: WmOutput) end)`
- `wm.outputs:on("removed", function(output: WmOutput) end)`: runs before the output is taken away, so its windows and `area` can still be asked about. Only virtual outputs are removed today: the tty backend does not handle unplugging

## WmOutput

- `output.name: string`
- `output.description: string`
- `output.properties: table`: `make`, `model`, `size`, `subpixel`
- `output:mode() -> OutputMode`
- `output:modes() -> OutputMode[]`
- `output:set_mode(mode: OutputMode) -> nil`
- `output:transform() -> "normal"|"90"|"180"|"270"|"flipped"|"flipped_90"|"flipped_180"|"flipped_270"`
- `output:set_transform(name: string) -> nil`
- `output:position() -> Point`
- `output:set_position(point: Point) -> nil`
- `output:scale() -> number`
- `output:set_scale(scale: number) -> nil`: 0.25 to 8
- `output:size() -> Size`: current mode, in physical pixels
- `output:area() -> Rect`: in the global layout
- `output:usable_area() -> Rect`: shortcut for `area` minus the exclusive zones of the layers on this output. It is what you get from `wm.layers:on_output(name)` by subtracting each layer's `exclusive_zone` plus the margin on its exclusive edge; `src/lua/tests/layers.rs` does that sum in Lua and checks it gives the same rect
- `output:add_shape(spec: ShapeSpec) -> Shape`: drawn on this output, at positions relative to its top left. The shapes go when the output does
- `output:remove() -> nil`: removes a virtual output; any other output raises

## OutputMode

- `mode.size: Size`: physical pixels
- `mode.refresh: number`: millihertz
- `mode.current: boolean`
- `mode.preferred: boolean`

## WmText

- `wm.text:measure(text: string, font: FontSpec?, max_width: number?) -> TextMetrics`: lays `text` out the way a text shape would, at scale 1, without drawing. Line breaks and tabs count as spaces, and text past 2048 characters is cut. With `max_width` the text is cut with an ellipsis ("…") until it fits

## TextMetrics

- `metrics.width: number`
- `metrics.height: number`
- `metrics.baseline: number`: from the top to the baseline
- `metrics.text: string`: what would be drawn

## FontSpec

A table. Fields left out keep their value when it is a shape update, and take the default when a shape is made.

- `family: string?`: a font family name, or `"sans-serif"`, `"serif"` or `"monospace"`. The default is `"sans-serif"`. Characters the family does not have come from other installed fonts
- `file: string?`: the path of a font file; used instead of `family`. A path that is not a font is an error when the text is drawn, and nothing is drawn
- `size: number?`: above 0, at most 1024, the default is 14. In logical pixels
- `weight: integer?`: 1 to 1000, the default is 400

## ShapeSpec

A table given to `window:add_shape` and `output:add_shape`, and to `shape:update` (without `kind`). A field that does not exist, or that does not belong to the kind, is an error. Colours are `"#rrggbb"` or `"#rrggbbaa"`.

- `kind: "rect"|"text"`: required when making a shape
- `x: integer?`, `y: integer?`: the top left, relative to the window or output. Default 0
- `z: integer?`: higher is in front. Default 0
- `layer: "background"|"bottom"|"top"|"overlay"?`: output shapes only, default `"top"`. Front to back the order is: overlay, fullscreen windows, top, other windows, bottom, background. Within one name, layer-shell surfaces are in front of shapes
- `visible: boolean?`: default true
- rect: `width: integer?`, `height: integer?` (0 to 16384), `fill: string|false?`, `radius: integer?` (clamped to half the shorter side), `border_width: integer?` (drawn inside the rectangle), `border_color: string?`
- text: `text: string?`, `font: FontSpec?`, `color: string?` (default white), `max_width: integer|false?`

Shapes are drawn into pixels at the output's scale, so they stay sharp at fractional scales. Nothing is redrawn until a shape changes. A shape made by a script is removed when that script is reloaded, so the new script makes its own.

## Shape

- `shape:update(spec: ShapeSpec) -> nil`: changes the given fields; raises when `spec` has `kind`, or a field of the wrong kind, and then changes nothing. Updating a removed shape does nothing
- `shape:remove() -> boolean`: true when the shape existed
- `shape:alive() -> boolean`
- `shape:kind() -> "rect"|"text"`
- `shape:size() -> Size?`: for text, the size it is drawn at; nil after removal

## WmInput

- `wm.input.seats: WmSeats`
- `wm.input:target_under(point: Point) -> Window|Layer|nil`

### WmInput events

- `wm.input:on("key", function(event: KeyEvent) end)`: also while the session is locked or locking, so check `wm.is_session_locked`. `prevent_default()` keeps the key from the focused client or the lock screen
- `wm.input:on("shortcuts_inhibit_request", function(inhibitor: ShortcutsInhibitor) end)`: a client asked to receive the compositor's shortcuts. Nothing is granted unless a handler calls `inhibitor:activate()`
- `wm.input:on("pointer_move", function(event: PointerMoveEvent) end)`
- `wm.input:on("pointer_button", function(event: PointerButtonEvent) end)`: before the client gets it
- `wm.input:on("pointer_button_forwarded", function(event: PointerButtonEvent) end)`: after it was delivered
- `wm.input:on("pointer_gesture", function(event: PointerGestureEvent) end)`
- `wm.input:on("pointer_constraint", function(constraint: PointerConstraint) end)`
- `wm.input:on("new_keyboard", function(keyboard: Keyboard) end)`
- `wm.input:on("new_pointer", function(pointer: Pointer) end)`

## WmSeats

- `wm.input.seats:count() -> integer`
- `wm.input.seats:all() -> Seat[]`
- `wm.input.seats:get(name: string) -> Seat?`

## Seat

- `seat.name: string`
- `seat:pointer() -> Pointer?`
- `seat:keyboard() -> Keyboard?`
- `seat:focused() -> Window|Layer|nil`: what this seat's keyboard is focused on
- `seat:focus(target: Window|Layer|nil) -> nil`: nil clears this seat's keyboard focus. Each seat has its own focus, so several windows can be focused at once; a hidden window cannot be focused

## Pointer

- `pointer:seat() -> Seat?`
- `pointer:position() -> Point`
- `pointer:buttons() -> integer[]`: pressed button codes
- `pointer:window_under() -> Window?`
- `pointer:move(point: Point) -> nil`
- `pointer:button(button: MouseButton, state: "pressed"|"released") -> nil`
- `pointer:scroll(horizontal: number, vertical: number, source: "wheel"|"finger"|"continuous"?) -> nil`: `source` defaults to `"wheel"`
- `pointer:gesture(kind: "swipe"|"pinch"|"hold", phase: "begin"|"update"|"end", options: table?) -> nil`: options are `fingers`, `delta`, `scale`, `rotation`, `cancelled`
- `pointer:grab() -> nil`: exclusive grab
- `pointer:ungrab() -> nil`
- `pointer:grabbed() -> boolean`

## Keyboard

- `keyboard:seat() -> Seat?`
- `keyboard:key(evdev_code: integer, state: "pressed"|"released") -> nil`: linux `input-event-codes.h` value, 30 is A
- `keyboard:get_layout() -> string`
- `keyboard:set_layout(keymap: string) -> boolean`
- `keyboard:repeat_rate() -> integer`
- `keyboard:repeat_delay() -> integer`
- `keyboard:set_repeat(rate: integer, delay: integer) -> nil`

## KeyEvent

- `event.keyboard: Keyboard`
- `event.state: "Pressed"|"Released"`
- `event.serial: number`
- `event.time: number`
- `event.keycode: number`
- `event.key: Keysym`
- `event.raw_key: Keysym`
- `event.keys: Keysym[]`
- `event.modifiers: Modifiers`
- `event.shortcuts_inhibited: boolean`
- `event:prevent_default() -> nil`

## PointerMoveEvent

- `event.pointer: Pointer`
- `event.position: Point`
- `event.output_position: Point?`
- `event.delta: Point`
- `event.delta_unaccel: Point`
- `event.modifiers: Modifiers`
- `event:prevent_default() -> nil`

## PointerButtonEvent

- `event.pointer: Pointer`
- `event.button: MouseButton`
- `event.state: "Pressed"|"Released"`
- `event.modifiers: Modifiers`
- `event:prevent_default() -> nil`

## PointerGestureEvent

- `event.pointer: Pointer`
- `event.kind: "swipe"|"pinch"|"hold"`
- `event.phase: "begin"|"update"|"end"`
- `event.fingers: number`
- `event.delta: Point`
- `event.scale: number`
- `event.rotation: number`
- `event.cancelled: boolean`
- `event.modifiers: Modifiers`
- `event:prevent_default() -> nil`

## PointerConstraint

- `constraint:alive() -> boolean`
- `constraint:kind() -> "locked"|"confined"?`: nil once the client destroyed it
- `constraint:active() -> boolean`
- `constraint:activate() -> nil`
- `constraint:deactivate() -> nil`
- `constraint:pointer() -> Pointer`
- `constraint:window() -> Window?`

## Modifiers

- `modifiers.shift: boolean`
- `modifiers.ctrl: boolean`
- `modifiers.alt: boolean`
- `modifiers.logo: boolean`
- `modifiers.caps_lock: boolean`
- `modifiers.num_lock: boolean`
- `modifiers.altgr: boolean`

## Keysym

- `Key.<name> -> Keysym`: xkb keysym name, case-insensitive
- `keysym == "name" | integer | Keysym`
- `tostring(keysym) -> string`
- `keysym .. string -> string`

## MouseButton

- `Button.Left`, `Button.Right`, `Button.Middle`, `Button.Back`, `Button.Forward`
- `button == "Left" | integer | MouseButton`
- `tostring(button) -> string`
- `button .. string -> string`
