# Marmot Server API

The Marmot HTTP server provides a simple API for rendering templates.

The server runs on port `3000` by default.

## Endpoints

### Health Check

**Endpoint:** `GET /health`

Returns a simple health check response.

**Request:**

```bash
curl http://localhost:3000/health
```

**Response:**

```
OK
```

---

### Render Template

**Endpoint:** `POST /render`

Renders a Marmot template with provided data and returns the rendered output file.

**Request:**

```bash
curl -X POST http://localhost:3000/render \
  -H "Content-Type: application/json" \
  -d '{
    "template": "label.marmot",
    "output": "png",
    "data": {
      "sku": "49000000001",
      "title": "Sample Product"
    }
  }' \
  --output rendered.png
```

**Accepts:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `template` | string | yes | `.marmot` template file name |
| `output` | string | yes | Output format: `"pdf"` or `"png"` |
| `data` | object | yes | JSON object with template slot values |
| `dpi` | number | no | PNG resolution in dots per inch (default: `300`, range: `72-1200`) |
| `dither` | string | no | Dither algorithm for PNG: `"floyd"`, `"atkinson"`, `"stucki"`, `"burkes"`, `"jarvis"`, `"sierra3"` |

**Response:**

Returns the rendered file as raw bytes with the appropriate `Content-Type`:

- `application/pdf` for PDF output
- `image/png` for PNG output

If rendering fails, returns a JSON error response:

```json
{
  "error": "Template 'label.marmot' not found"
}
```

