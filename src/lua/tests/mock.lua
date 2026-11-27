local function event_source()
	local source = { handlers = {} }
	function source:on(name, handler)
		self.handlers[name] = self.handlers[name] or {}
		table.insert(self.handlers[name], handler)
	end
	function source:emit(name, ...)
		for _, handler in ipairs(self.handlers[name] or {}) do handler(...) end
	end
	return source
end

local function rect(x, y, width, height)
	return { x = x, y = y, width = width, height = height }
end

Key = setmetatable({}, { __index = function(_, name) return name end })
Button = { Left = "Left", Right = "Right", Middle = "Middle" }
function spawn() end

local windows = {}
local seats = {}
local stacking = {}
local next_id = 1

local output_scale = 1
local output_list = {}

local function mock_shape(owner, spec)
	local shape = { spec = {}, removed = false, updates = 0 }
	for key, value in pairs(spec) do shape.spec[key] = value end
	function shape:update(change)
		for key, value in pairs(change) do self.spec[key] = value end
		self.updates = self.updates + 1
	end
	function shape:remove()
		self.removed = true
		remove_item(owner.shapes, self)
		return true
	end
	function shape:alive() return not self.removed end
	function shape:kind() return self.spec.kind end
	table.insert(owner.shapes, shape)
	return shape
end

function remove_item(list, item)
	for i, other in ipairs(list) do
		if other == item then table.remove(list, i) return end
	end
end

function new_output(name, x, y, width, height)
	local output = { name = name, shapes = {}, box = rect(x, y, width, height) }
	function output:scale() return output_scale end
	function output:area() return self.box end
	function output:usable_area() return self.usable or self.box end
	function output:position() return Point(self.box.x, self.box.y) end
	function output:add_shape(spec) return mock_shape(self, spec) end
	table.insert(output_list, output)
	if wm and wm.outputs then wm.outputs:emit("added", output) end
	return output
end

function remove_output(output)
	wm.outputs:emit("removed", output)
	remove_item(output_list, output)
end

local first_output = new_output("mock-1", 0, 0, 1000, 840)
first_output.usable = rect(0, 0, 1000, 800)
local output = first_output

wm = event_source()
wm.input = event_source()
wm.windows = event_source()
wm.layers = event_source()
wm.outputs = event_source()
wm.text = {}
function wm.text:measure(text, font, max_width)
	local width = #text * 7
	local shown = text
	if max_width and width > max_width then
		shown = text:sub(1, math.max(math.floor(max_width / 7) - 1, 0)) .. "…"
		width = (#shown - 2) * 7
	end
	return { width = width, height = 16, baseline = 12, text = shown }
end
wm.is_session_locked = false
wm.session_lock = { backdrop_color = nil }
function wm.session_lock:set_backdrop_color(color) self.backdrop_color = color end
function wm.outputs:all() return output_list end
function wm.outputs:count() return #output_list end
function wm.outputs:get(index) return output_list[index] end
function wm.outputs:name(name)
	for _, candidate in ipairs(output_list) do
		if candidate.name == name then return candidate end
	end
end
function wm.outputs:under(point)
	local found = {}
	for _, candidate in ipairs(output_list) do
		local box = candidate.box
		if point.x >= box.x and point.x < box.x + box.width and point.y >= box.y and point.y < box.y + box.height then
			table.insert(found, candidate)
		end
	end
	return found
end
function wm:quit() end
wm.wayland_display = "wayland-test"
function set_output_scale(scale)
	output_scale = scale
	wm.outputs:emit("changed", output)
end
wm.imported = {}
function wm:import_environment(variables)
	table.insert(wm.imported, variables)
	return {}
end

under = nil

wm.store = {}
wm.is_reload = false
wm.input.seats = {}
function wm.input.seats:all() return seats end
function wm.input.seats:count() return #seats end
function wm.input.seats:get(name)
	for _, seat in ipairs(seats) do
		if seat.name == name then return seat end
	end
end
function wm:reload() reload_requested = true end

function wm.windows:by_id(id)
	for _, window in ipairs(windows) do
		if window:id() == id then return window end
	end
end
function wm.windows:all() return { table.unpack(stacking) } end
function wm.windows:under() return under and under:kind() == "window" and under or nil end
function wm.windows:count() return #windows end
function wm.windows:topmost()
	for i = #stacking, 1, -1 do
		if stacking[i].visible then return stacking[i] end
	end
end
function new_seat(name)
	local seat = { name = name, target = nil }
	function seat:keyboard() return nil end
	function seat:focused() return self.target end
	function seat:focus(target)
		if target and target:kind() == "window" and not target.visible then return end
		self.target = target
		wm.windows:emit("focus", target and target:kind() == "window" and target or nil, self)
	end
	table.insert(seats, seat)
	return seat
end
seat0 = new_seat("seat0")

local function seats_focusing(window)
	local found = {}
	for _, seat in ipairs(seats) do
		if seat.target == window then table.insert(found, seat) end
	end
	return found
end
function wm.input:target_under() return under end

local function remove(list, item)
	for i, other in ipairs(list) do
		if other == item then table.remove(list, i) return end
	end
end

function new_window(name)
	local window = { name = name, visible = true, x = 0, y = 0, width = 0, height = 0, flags = {} }
	local id = next_id
	next_id = next_id + 1
	function window:id() return id end
	function window:kind() return "window" end
	function window:position() return Point(self.x, self.y) end
	function window:size() return { width = self.width, height = self.height } end
	function window:set_position(point) self.x, self.y = point.x, point.y end
	function window:set_size(size) self.width, self.height = size.width, size.height end
	function window:clear_size() self.width, self.height = 0, 0 end
	function window:output()
		local found = wm.outputs:under(Point(self.x, self.y))[1]
		return found or output_list[1]
	end
	window.shapes = {}
	function window:add_shape(spec) return mock_shape(self, spec) end
	function window:title() return self.name end
	function window:parent() return self.parent_window end
	function window:set_decoration(mode) self.decoration = mode end
	function window:mapped() return self.is_mapped == true end
	function window:set_border(width, color) self.border = { width = width, color = color } end
	function window:clear_border() self.border = nil end
	function window:layer() return self.depth or "normal" end
	function window:set_layer(depth) self.depth = depth end
	function window:app_id() return self.app end
	function window:close() self.close_requested = true end
	function window:raise()
		remove(stacking, self)
		table.insert(stacking, self)
	end
	function window:set_visible(visible)
		self.visible = visible
		if visible then return end
		for _, seat in ipairs(seats_focusing(self)) do seat:focus(nil) end
	end
	function window:focused_seats() return seats_focusing(self) end
	function window:set_activated(active) self.activated = active end
	function window:states()
		local owner = self
		return {
			has = function(_, flag) return owner.flags[flag] == true end,
			set = function(_, flag, enabled) owner.flags[flag] = enabled or nil end,
		}
	end
	table.insert(windows, window)
	table.insert(stacking, window)
	wm.windows:emit("new_window", window)
	return window
end

function map_window(name, app_id)
	local window = new_window(name)
	window.app = app_id
	window.is_mapped = true
	wm.windows:emit("mapped", window)
	return window
end

local function plain_copy(value, path)
	if type(value) == "table" then
		local copy = {}
		for key, inner in pairs(value) do copy[key] = plain_copy(inner, path .. "." .. tostring(key)) end
		return copy
	elseif type(value) == "function" or type(value) == "userdata" then
		error(path .. " cannot be stored")
	end
	return value
end

function reload()
	wm:emit("unload")
	local store = plain_copy(wm.store, "wm.store")
	for _, source in ipairs({ wm, wm.input, wm.windows, wm.layers, wm.outputs }) do source.handlers = {} end
	for name in pairs(package.preload) do package.loaded[name] = nil end
	wm.store = store
	wm.is_reload = true
	require("main")
end

function close_window(window)
	remove(windows, window)
	remove(stacking, window)
	for _, seat in ipairs(seats_focusing(window)) do seat.target = nil end
	wm.windows:emit("closed", window)
end

local layer_list = {}

function wm.layers:all() return layer_list end
function wm.layers:on_output(name)
	local found = {}
	for _, layer in ipairs(layer_list) do
		if layer.output_name == name then table.insert(found, layer) end
	end
	return found
end

function new_layer(depth, interactivity, properties)
	properties = properties or {}
	local layer = { properties = properties, output_name = properties.output or output_list[1].name }
	function layer:kind() return "layer" end
	function layer:layer() return depth end
	function layer:keyboard_interactivity() return interactivity end
	function layer:anchor()
		local anchor = self.properties.anchor or {}
		return { top = anchor.top or false, bottom = anchor.bottom or false, left = anchor.left or false, right = anchor.right or false }
	end
	function layer:margin()
		local margin = self.properties.margin or {}
		return { top = margin.top or 0, right = margin.right or 0, bottom = margin.bottom or 0, left = margin.left or 0 }
	end
	function layer:exclusive_zone() return self.properties.exclusive_zone or 0 end
	function layer:exclusive_edge() return self.properties.exclusive_edge end
	function layer:requested_size() return self.properties.size or { width = 0, height = 0 } end
	function layer:output() return wm.outputs:name(self.output_name) end
	table.insert(layer_list, layer)
	wm.layers:emit("new_surface", layer)
	wm.layers:emit("mapped", layer)
	return layer
end

function change_layer(layer, properties)
	for key, value in pairs(properties) do layer.properties[key] = value end
	wm.layers:emit("changed", layer)
end

function close_layer(layer)
	remove_item(layer_list, layer)
	wm.layers:emit("remove_surface", layer)
	for _, seat in ipairs(seats) do
		if seat.target == layer then seat.target = nil end
	end
end

function new_pointer(x, y, seat)
	local pointer = { x = x, y = y, held = {}, is_grabbed = false }
	function pointer:seat() return seat or seats[1] end
	function pointer:position() return Point(self.x, self.y) end
	function pointer:buttons() return self.held end
	function pointer:grab() self.is_grabbed = true end
	function pointer:ungrab() self.is_grabbed = false end
	function pointer:grabbed() return self.is_grabbed end
	function pointer:window_under()
		return under and under:kind() == "window" and under or nil
	end
	return pointer
end

function new_constraint(pointer, window)
	local constraint = { is_alive = true, is_active = false }
	function constraint:alive() return self.is_alive end
	function constraint:active() return self.is_active end
	function constraint:activate() self.is_active = true end
	function constraint:deactivate() self.is_active = false end
	function constraint:pointer() return pointer end
	function constraint:window() return window end
	wm.input:emit("pointer_constraint", constraint)
	return constraint
end

function press_button(pointer, state, button, modifiers)
	if state == "Pressed" then
		table.insert(pointer.held, button)
	else
		remove(pointer.held, button)
	end
	local event = { state = state, button = button, pointer = pointer, modifiers = modifiers or {} }
	function event:prevent_default() self.prevented = true end
	wm.input:emit("pointer_button", event)
	if not event.prevented then wm.input:emit("pointer_button_forwarded", event) end
	return event
end

function move_pointer(pointer, x, y)
	pointer.x, pointer.y = x, y
	wm.input:emit("pointer_move", { position = Point(x, y), pointer = pointer })
end

function key_event(state, key, modifiers, keycode, seat)
	local keyboard = { seat = function() return seat or seats[1] end }
	local event = { state = state, raw_key = key, modifiers = modifiers or {}, keycode = keycode or 1, keyboard = keyboard }
	function event:prevent_default() self.prevented = true end
	return event
end
