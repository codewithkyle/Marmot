local regular_price = data.getSlot("regular_price") or 0
local get_qty = data.getSlot("get_qty") or 0
local savings = regular_price * get_qty

if savings < 25.0 then
    frame.visible = false
end
