local tiling = require("tiling")
local outputs = require("outputs")
local M = {}

local style = {
	height = 28,
	pill_height = 20,
	pill_padding = 8,
	gap = 4,
	margin = 6,
	radius = 4,
	background = "#2e3440",
	font = { size = 13 },
	title_color = "#d8dee9",
	pills = {
		here = { fill = "#88c0d0", text = "#2e3440" },
		elsewhere = { border = "#88c0d0", text = "#88c0d0" },
		occupied = { fill = "#434c5e", text = "#d8dee9" },
		empty = { text = "#4c566a" },
	},
}

local bars = {}
local gone = {}

local function state_of(info, output)
	if info.output == output.name then return "here" end
	if info.output then return "elsewhere" end
	return info.count > 0 and "occupied" or "empty"
end

local function focused_title_on(output)
	local seat = wm.input.seats:get("seat0")
	local target = seat and seat:focused()
	if not (target and target:kind() == "window") then return "" end
	local on = target:output()
	if not (on and on.name == output.name) then return "" end
	return target:title() or target:app_id() or ""
end

local function placement(output)
	local area, usable = output:area(), output:usable_area()
	return { x = usable.x - area.x, y = usable.y - area.y, width = usable.width }
end

local function pill_layout(output, origin)
	local pills, x = {}, origin.x + style.margin
	for _, info in ipairs(tiling.list()) do
		local metrics = wm.text:measure(info.name, style.font)
		local width = math.ceil(metrics.width) + 2 * style.pill_padding
		table.insert(pills, { name = info.name, state = state_of(info, output), x = x, width = width, text_width = metrics.width, text_height = metrics.height })
		x = x + width + style.gap
	end
	return pills, x
end

local function signature(origin, pills, title)
	local parts = { origin.x, origin.y, origin.width, title }
	for _, pill in ipairs(pills) do table.insert(parts, pill.name .. pill.state) end
	return table.concat(parts, "|")
end

local function draw_pills(output, bar, pills, origin)
	for _, shape in ipairs(bar.pills) do shape:remove() end
	bar.pills = {}
	local top = origin.y + (style.height - style.pill_height) // 2
	for _, pill in ipairs(pills) do
		local look = style.pills[pill.state]
		table.insert(bar.pills, output:add_shape({
			kind = "rect", x = pill.x, y = top, width = pill.width, height = style.pill_height, radius = style.radius,
			fill = look.fill or false, border_width = look.border and 1 or 0, border_color = look.border, z = 1,
		}))
		table.insert(bar.pills, output:add_shape({
			kind = "text", x = pill.x + (pill.width - math.floor(pill.text_width)) // 2,
			y = top + (style.pill_height - math.floor(pill.text_height)) // 2,
			text = pill.name, font = style.font, color = look.text, z = 2,
		}))
	end
end

local function create(output)
	outputs.reserve_top(output, style.height)
	return {
		signature = "",
		pills = {},
		background = output:add_shape({ kind = "rect", x = 0, y = 0, width = 0, height = style.height, fill = style.background, layer = "top", z = 0 }),
		title = output:add_shape({ kind = "text", x = 0, y = 0, text = "", font = style.font, color = style.title_color, layer = "top", z = 2, visible = false }),
	}
end

local function sync(output)
	local bar = bars[output.name]
	if not bar then
		bar = create(output)
		bars[output.name] = bar
	end
	local origin = placement(output)
	local pills, after_pills = pill_layout(output, origin)
	local title = focused_title_on(output)
	local now = signature(origin, pills, title)
	if now == bar.signature then return end
	bar.signature = now
	bar.background:update({ x = origin.x, y = origin.y, width = origin.width })
	draw_pills(output, bar, pills, origin)
	local title_x = after_pills + 2 * style.margin
	local metrics = wm.text:measure("Ag", style.font)
	bar.title:update({
		x = title_x, y = origin.y + (style.height - math.floor(metrics.height)) // 2, text = title,
		max_width = math.max(origin.x + origin.width - title_x - style.margin, 0), visible = title ~= "",
	})
end

--- Brings every output's bar in line with the workspaces and the focused window; a bar whose content is unchanged is left alone.
function M.refresh()
	for _, output in ipairs(wm.outputs:all()) do
		if not gone[output.name] then sync(output) end
	end
end

wm.outputs:on("added", function(output)
	gone[output.name] = nil
	tiling.retile()
end)
wm.outputs:on("changed", M.refresh)
wm.outputs:on("removed", function(output)
	gone[output.name] = true
	bars[output.name] = nil
end)
wm.windows:on("focus", M.refresh)
wm.windows:on("title_changed", M.refresh)
wm.windows:on("closed", M.refresh)
wm.layers:on("mapped", M.refresh)
wm.layers:on("changed", M.refresh)
wm.layers:on("remove_surface", M.refresh)
tiling.on_change(M.refresh)

M.refresh()
tiling.retile()

return M
