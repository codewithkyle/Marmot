local regular_price = data.getSlot("regular_price") or 0
local get_qty = data.getSlot("get_qty") or 0
local savings = save_amount(regular_price * get_qty, 0)

if savings >= 25.0 then
    frame.text_color = parse_rgb("0 0 0")
else
    frame.text_color = parse_rgb("0.92 0.07 0.16")
end
