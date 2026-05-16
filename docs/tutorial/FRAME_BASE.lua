local regular_price = data.getSlot("regular_price") or 0
local get_qty = data.getSlot("get_qty") or 0
local savings = save_amount(regular_price * get_qty, 0)

if savings >= 25.0 then
    frame.fill_color = parse_rgb("0.92 0.07 0.16")
    frame.stroke_color = parse_rgb("1 1 1")
else
    frame.fill_color = parse_rgb("1 1 1")
    frame.stroke_color = cmyk_to_rgb(0, 0, 0, 0.08)
end

frame.stroke_width = 6
