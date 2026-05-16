local regular_price = data.getSlot("regular_price") or 0
local get_qty = data.getSlot("get_qty") or 0
local savings = save_amount(regular_price * get_qty, 0)

if savings >= 25.0 then
    frame.value = "super_save"
elseif regular_price >= 5.0 then
    frame.value = "save_5"
else
    frame.visible = false
end
