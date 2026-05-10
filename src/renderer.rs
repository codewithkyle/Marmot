use crate::parser::{DrawOp, NumberValue};

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
}
