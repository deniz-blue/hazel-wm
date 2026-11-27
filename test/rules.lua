local tiling = require("tiling")
local floating = require("floating")

local M = {}

local rules = {}

local function matches(rule, window)
	if rule.app_id and not (window:app_id() or ""):find(rule.app_id) then return false end
	if rule.title and not (window:title() or ""):find(rule.title) then return false end
	return true
end

local function apply(rule, window)
	if rule.workspace then tiling.move_window(window, rule.workspace) end
	if rule.floating ~= nil then floating.set_floating(window, rule.floating) end
	if rule.layer then window:set_layer(rule.layer) end
	if rule.run then rule.run(window) end
end

--- `app_id` and `title` are Lua patterns. Matching windows get `workspace`,
--- `floating`, `layer`, and `run(window)` applied when they first appear.
function M.add(rule)
	table.insert(rules, rule)
end

wm.windows:on("mapped", function(window)
	for _, rule in ipairs(rules) do
		if matches(rule, window) then apply(rule, window) end
	end
end)

return M
