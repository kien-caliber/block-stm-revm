//! SVG

use std::{thread::ThreadId, time::Instant};

use dashmap::DashMap;

use crate::Task;

struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    /// Constructor for the Rect struct
    fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Rect { x0, y0, x1, y1 }
    }

    /// Union of two [Rect] values
    fn union_two(a: &Self, b: &Self) -> Self {
        Rect {
            x0: a.x0.min(b.x0),
            y0: a.y0.min(b.y0),
            x1: a.x1.max(b.x1),
            y1: a.y1.max(b.y1),
        }
    }

    /// Union of multiple [Rect] values
    fn union<'a>(items: impl Iterator<Item = &'a Self>) -> Self {
        items.fold(
            Self::new(f64::INFINITY, f64::INFINITY, -f64::INFINITY, -f64::INFINITY),
            |acc, rect| Self::union_two(&acc, rect),
        )
    }

    fn to_svg_rect(&self, title: &str, hue: f64) -> String {
        [
            format!("<rect",),
            format!(" x='{}'", self.x0),
            format!(" y='{}'", self.y0),
            format!(" width='{}'", self.x1 - self.x0),
            format!(" height='{}'", self.y1 - self.y0),
            format!(" style='fill: hsl({}, 50%, 50%)'", hue),
            format!(">"),
            format!("<title>{}</title>", title),
            format!("</rect>"),
        ]
        .join("")
    }

    fn to_ratio_rect(&self, bounds: &Self) -> Self {
        Self {
            x0: (self.x0 - bounds.x0) / (bounds.x1 - bounds.x0),
            y0: (self.y0 - bounds.y0) / (bounds.y1 - bounds.y0),
            x1: (self.x1 - bounds.x0) / (bounds.x1 - bounds.x0),
            y1: (self.y1 - bounds.y0) / (bounds.y1 - bounds.y0),
        }
    }
}

/// Inspector
#[derive(Debug, Default)]
pub struct Inspector {
    thread_to_events: DashMap<ThreadId, Vec<(Task, Instant, Instant)>>,
}

impl Inspector {
    pub(crate) fn clear(&mut self) {
        self.thread_to_events.clear();
    }

    pub(crate) fn record(&self, task: Task, t0: Instant, t1: Instant) {
        self.thread_to_events
            .entry(std::thread::current().id())
            .or_default()
            .push((task, t0, t1));
    }

    pub(crate) fn measure<R>(&self, task: Task, f: impl FnOnce() -> R) -> R {
        let t0 = Instant::now();
        let result = f();
        let t1 = Instant::now();
        self.record(task, t0, t1);
        result
    }

    pub(crate) fn to_svg(&self, created_at: Instant, dropped_at: Instant) -> String {
        let thread_ids: Vec<ThreadId> = {
            let mut sequence: Vec<_> = self
                .thread_to_events
                .iter()
                .map(|r| {
                    (
                        r.key().clone(),
                        r.value()
                            .iter()
                            .map(|(_, t0, _)| t0.duration_since(created_at))
                            .min()
                            .unwrap_or_default(),
                    )
                })
                .collect();
            sequence.sort_by_key(|(_, t)| *t);
            sequence.into_iter().map(|(t, _)| t).collect()
        };

        let mut rects_3 = Vec::new();

        for ref_ in self.thread_to_events.iter() {
            let (thread_id, events) = ref_.pair();
            let thread_number = thread_ids.iter().position(|t| t == thread_id).unwrap();
            for (task, t0, t1) in events {
                let w = match task {
                    Task::Execution(_) => 0.8,
                    Task::Validation(_) => 0.08,
                };
                let x0 = thread_number as f64 - w / 2.0;
                let x1 = thread_number as f64 + w / 2.0;
                let y0 = t0.duration_since(created_at).as_secs_f64();
                let y1 = t1.duration_since(created_at).as_secs_f64();
                let label = format!("{:?}", task);
                let hue = match task {
                    Task::Execution(tx_version) => {
                        f64::powi(0.5, tx_version.tx_incarnation as i32) * 120.0
                    }
                    Task::Validation(tx_version) => {
                        360.0 - f64::powi(0.5, tx_version.tx_incarnation as i32) * 120.0
                    }
                };
                rects_3.push((Rect::new(x0, y0, x1, y1), label, hue));
            }
        }

        let bounding_rect =
            Rect::union(rects_3.iter().map(|(rect, _, _)| rect).chain(&[Rect::new(
                0 as f64,
                0 as f64,
                thread_ids.len() as f64,
                dropped_at.duration_since(created_at).as_secs_f64(),
            )]));

        let mut lines: Vec<String> = Vec::new();
        lines.push("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1' width='100%' height='100%' preserveAspectRatio='none'>".to_string());
        lines.push("<style>rect:hover { opacity: 0.5; }</style>".to_string());
        for (rect, label, hue) in rects_3 {
            lines.push(rect.to_ratio_rect(&bounding_rect).to_svg_rect(&label, hue));
        }
        lines.push("</svg>".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_two_no_overlap() {
        let rect1 = Rect::new(0.0, 0.0, 1.0, 1.0);
        let rect2 = Rect::new(2.0, 2.0, 3.0, 3.0);
        let union_rect = Rect::union_two(&rect1, &rect2);

        assert_eq!(union_rect.x0, 0.0);
        assert_eq!(union_rect.y0, 0.0);
        assert_eq!(union_rect.x1, 3.0);
        assert_eq!(union_rect.y1, 3.0);
    }

    #[test]
    fn test_union_two_with_overlap() {
        let rect1 = Rect::new(0.0, 0.0, 2.0, 2.0);
        let rect2 = Rect::new(1.0, 1.0, 3.0, 3.0);
        let union_rect = Rect::union_two(&rect1, &rect2);

        assert_eq!(union_rect.x0, 0.0);
        assert_eq!(union_rect.y0, 0.0);
        assert_eq!(union_rect.x1, 3.0);
        assert_eq!(union_rect.y1, 3.0);
    }

    #[test]
    fn test_union_multiple_rects() {
        let rect1 = Rect::new(0.0, 0.0, 1.0, 1.0);
        let rect2 = Rect::new(1.0, 1.0, 3.0, 3.0);
        let rect3 = Rect::new(-1.0, -1.0, 0.5, 0.5);

        let rects = vec![rect1, rect2, rect3];
        let union_rect = Rect::union(rects.iter());

        assert_eq!(union_rect.x0, -1.0);
        assert_eq!(union_rect.y0, -1.0);
        assert_eq!(union_rect.x1, 3.0);
        assert_eq!(union_rect.y1, 3.0);
    }

    #[test]
    fn test_union_single_rect() {
        let rect = Rect::new(0.0, 0.0, 2.0, 2.0);
        let union_rect = Rect::union([rect].iter());

        assert_eq!(union_rect.x0, 0.0);
        assert_eq!(union_rect.y0, 0.0);
        assert_eq!(union_rect.x1, 2.0);
        assert_eq!(union_rect.y1, 2.0);
    }

    #[test]
    fn test_union_empty() {
        let rects: Vec<Rect> = Vec::new();
        let union_rect = Rect::union(rects.iter());

        assert_eq!(union_rect.x0, f64::INFINITY);
        assert_eq!(union_rect.y0, f64::INFINITY);
        assert_eq!(union_rect.x1, -f64::INFINITY);
        assert_eq!(union_rect.y1, -f64::INFINITY);
    }
}
