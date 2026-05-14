# Frames Block (Draft)

This draft captures current direction for minimal `frames` block in `.psl`.

## Goals

- Keep render IR compact and machine-friendly.
- Keep external data contract (`slots`) separate from frame runtime state.
- Keep designer metadata out of render IR.
- Support stable identity across designer edits.

## Proposed Row Shape

Each frame row contains only packed index + stable identity:

```text
<u32> <ULID>
```

- `<u32>`: generated at pack/export time; used by draw ops (`frame <u32>`).
- `<ULID>`: stable internal identity across edits/re-packs; script key (`scripts/<ULID>.lua`).

Notes:

- `u32` can change on repack; `ULID` should not.
- Designer metadata (label, x/y/w/h, type) lives in `designer.json`.

## Example `.psl` Layout

```psl
version 1

slots
    upc string optional
    logo_asset string optional
end

frames
    1 01J8Z7ABCDEFGHJKMNPQRS0
    2 01J8Z8ABCDEFGHJKMNPQRS1
    3 01J8Z9ABCDEFGHJKMNPQRS2
end

draw
    frame 1
        0 0 0 rgb
        0 0 100 20 rect fill
    end

    frame 2
        (logo) 0 0 100 100 image
    end

    frame 3
        (upc) 0 0 120 24 barcode
    end
end
```

## Draw Block Usage

- Draw block references packed frame index: `frame <u32>`.
- Renderer maps `u32 -> ULID`, then checks `scripts/<ULID>.lua`.
- If script missing, default `visible = true`.
- If script exists, execute and apply output (`frame.visible`, `frame.value`).
- Script/type errors fail render hard.
