---@meta

---Spawn a process and forget about it
---@param command string
---@param args string[] | nil
function spawn(command, args) end

---Table of keysyms for easy access
---@type table<string, Keysym>
Key = {}

---@class WmButton
local WmButton = {}

---Table of buttons
---@class Button
---@field Left WmButton
---@field Right WmButton
---@field Middle WmButton
---@field Back WmButton
---@field Forward WmButton
Button = {}
