use std::path::Path;

use crate::parser::{DrawOp, NumberValue, Page};
use cairo::{Context, PdfSurface};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderOp {
    SetRgb {
        r: f64,
        g: f64,
        b: f64,
    },
    SetStrokeWidth {
        width: f64,
    },
    StrokeLine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    StrokeRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    FillRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

#[derive(Debug)]
pub enum RenderError {
    Cario(cairo::Error),
    Io(std::io::Error),
}

impl From<cairo::Error> for RenderError {
    fn from(err: cairo::Error) -> Self {
        RenderError::Cario(err)
    }
}

impl From<std::io::Error> for RenderError {
    fn from(err: std::io::Error) -> Self {
        RenderError::Io(err)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PendingPath {
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

fn eval_number(value: &NumberValue) -> f64 {
    match value {
        NumberValue::Literal(n) => *n,
        NumberValue::Slot(name) => {
            panic!("not supported {name:?}")
        }
    }
}

pub fn lower_draw_ops(draw_ops: &[DrawOp]) -> Vec<RenderOp> {
    let mut render_ops = Vec::new();
    let mut pending_path: Option<PendingPath> = None;

    for draw_op in draw_ops {
        match draw_op {
            DrawOp::SetRgb { r, g, b } => {
                render_ops.push(RenderOp::SetRgb {
                    r: eval_number(r),
                    g: eval_number(g),
                    b: eval_number(b),
                });
            }
            DrawOp::SetStrokeWidth { width } => {
                render_ops.push(RenderOp::SetStrokeWidth {
                    width: eval_number(width),
                });
            }
            DrawOp::LinePath { x1, y1, x2, y2 } => {
                pending_path = Some(PendingPath::Line {
                    x1: eval_number(x1),
                    y1: eval_number(y1),
                    x2: eval_number(x2),
                    y2: eval_number(y2),
                });
            }
            DrawOp::RectPath {
                x,
                y,
                width,
                height,
            } => {
                pending_path = Some(PendingPath::Rect {
                    x: eval_number(x),
                    y: eval_number(y),
                    width: eval_number(width),
                    height: eval_number(height),
                });
            }
            DrawOp::Stroke => {
                let path = pending_path
                    .take()
                    .expect("parser should prevent stroke without a current path");
                match path {
                    PendingPath::Line { x1, y1, x2, y2 } => {
                        render_ops.push(RenderOp::StrokeLine { x1, y1, x2, y2 });
                    }
                    PendingPath::Rect {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        render_ops.push(RenderOp::StrokeRect {
                            x,
                            y,
                            width,
                            height,
                        });
                    }
                }
            }
            DrawOp::Fill => {
                let path = pending_path
                    .take()
                    .expect("parser should prevent fill without a current path");
                match path {
                    PendingPath::Rect {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        render_ops.push(RenderOp::FillRect {
                            x,
                            y,
                            width,
                            height,
                        });
                    }
                    PendingPath::Line { .. } => {
                        panic!("parser should prevent filling a line");
                    }
                }
            }
            DrawOp::TextBox { .. } => {
                todo!("textbox rendering will be added later");
            }
        }
    }

    render_ops
}

fn execute_render_ops(ctx: &Context, render_ops: &[RenderOp]) -> Result<(), RenderError> {
    for op in render_ops {
        match op {
            RenderOp::SetRgb { r, g, b } => {
                ctx.set_source_rgb(*r, *g, *b);
            }
            RenderOp::SetStrokeWidth { width } => {
                ctx.set_line_width(*width);
            }
            RenderOp::StrokeLine { x1, y1, x2, y2 } => {
                ctx.move_to(*x1, *y1);
                ctx.line_to(*x2, *y2);
                ctx.stroke()?;
            }
            RenderOp::StrokeRect {
                x,
                y,
                width,
                height,
            } => {
                ctx.rectangle(*x, *y, *width, *height);
                ctx.stroke()?;
            }
            RenderOp::FillRect {
                x,
                y,
                width,
                height,
            } => {
                ctx.rectangle(*x, *y, *width, *height);
                ctx.fill()?;
            }
            _ => todo!("execute render op"),
        }
    }

    Ok(())
}

pub fn render_pdf(page: &Page, draw_ops: &[DrawOp], output_path: &Path) -> Result<(), RenderError> {
    let render_ops = lower_draw_ops(draw_ops);

    let surface = PdfSurface::new(page.width, page.height, output_path)?;
    let ctx = Context::new(&surface)?;

    execute_render_ops(&ctx, &render_ops)?;

    surface.finish();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{DrawOp, NumberValue};

    #[test]
    fn lowers_rect_fill() {
        let draw_ops = vec![
            DrawOp::RectPath {
                x: NumberValue::Literal(10.0),
                y: NumberValue::Literal(20.0),
                width: NumberValue::Literal(30.0),
                height: NumberValue::Literal(40.0),
            },
            DrawOp::Fill,
        ];

        let render_ops = lower_draw_ops(&draw_ops);

        assert_eq!(
            render_ops,
            vec![RenderOp::FillRect {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0,
            }]
        );
    }

    #[test]
    fn lowers_rect_stroke() {
        let draw_ops = vec![
            DrawOp::RectPath {
                x: NumberValue::Literal(10.0),
                y: NumberValue::Literal(20.0),
                width: NumberValue::Literal(30.0),
                height: NumberValue::Literal(40.0),
            },
            DrawOp::Stroke,
        ];

        let render_ops = lower_draw_ops(&draw_ops);

        assert_eq!(
            render_ops,
            vec![RenderOp::StrokeRect {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0,
            }]
        );
    }

    #[test]
    fn lowers_line_stroke() {
        let draw_ops = vec![
            DrawOp::LinePath {
                x1: NumberValue::Literal(0.0),
                y1: NumberValue::Literal(0.0),
                x2: NumberValue::Literal(100.0),
                y2: NumberValue::Literal(100.0),
            },
            DrawOp::Stroke,
        ];

        let render_ops = lower_draw_ops(&draw_ops);

        assert_eq!(
            render_ops,
            vec![RenderOp::StrokeLine {
                x1: 0.0,
                y1: 0.0,
                x2: 100.0,
                y2: 100.0,
            }]
        );
    }

    #[test]
    fn renders_basic_pdf() {
        use crate::parser::{DrawOp, NumberValue, Page};
        use std::fs;

        let page = Page {
            width: 200.0,
            height: 200.0,
        };

        let draw_ops = vec![
            DrawOp::SetRgb {
                r: NumberValue::Literal(1.0),
                g: NumberValue::Literal(1.0),
                b: NumberValue::Literal(1.0),
            },
            DrawOp::RectPath {
                x: NumberValue::Literal(0.0),
                y: NumberValue::Literal(0.0),
                width: NumberValue::Literal(200.0),
                height: NumberValue::Literal(200.0),
            },
            DrawOp::Fill,
            DrawOp::SetRgb {
                r: NumberValue::Literal(1.0),
                g: NumberValue::Literal(0.0),
                b: NumberValue::Literal(0.0),
            },
            DrawOp::RectPath {
                x: NumberValue::Literal(20.0),
                y: NumberValue::Literal(20.0),
                width: NumberValue::Literal(80.0),
                height: NumberValue::Literal(60.0),
            },
            DrawOp::Fill,
            DrawOp::SetRgb {
                r: NumberValue::Literal(0.0),
                g: NumberValue::Literal(0.0),
                b: NumberValue::Literal(0.0),
            },
            DrawOp::SetStrokeWidth {
                width: NumberValue::Literal(2.0),
            },
            DrawOp::RectPath {
                x: NumberValue::Literal(20.0),
                y: NumberValue::Literal(20.0),
                width: NumberValue::Literal(80.0),
                height: NumberValue::Literal(60.0),
            },
            DrawOp::Stroke,
            DrawOp::LinePath {
                x1: NumberValue::Literal(0.0),
                y1: NumberValue::Literal(0.0),
                x2: NumberValue::Literal(200.0),
                y2: NumberValue::Literal(200.0),
            },
            DrawOp::Stroke,
        ];

        let output_path = std::env::temp_dir().join("marmot_basic_render_test.pdf");

        render_pdf(&page, &draw_ops, &output_path).unwrap();

        let metadata = fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);

        let _ = fs::remove_file(output_path);
    }
}
