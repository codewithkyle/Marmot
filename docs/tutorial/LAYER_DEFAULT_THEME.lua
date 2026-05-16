local regular_price = data.getSlot("regular_price") or 0
local get_qty = data.getSlot("get_qty") or 0
local savings = save_amount(regular_price * get_qty, 0)

if savings >= 25.0 then
    layer.visible = false
end
