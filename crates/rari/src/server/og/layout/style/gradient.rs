use image::Rgba;

#[derive(Debug, Clone, Copy)]
pub enum StopPosition {
    Percentage(f32),
    Px(f32),
    Normalized(f32),
}

impl StopPosition {
    pub fn to_px(self, axis_length: f32) -> f32 {
        match self {
            StopPosition::Percentage(pct) => (pct / 100.0) * axis_length,
            StopPosition::Px(px) => px,
            StopPosition::Normalized(n) => n * axis_length,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorStop {
    pub color: Rgba<u8>,
    pub position: Option<StopPosition>,
}

#[derive(Debug, Clone)]
pub struct ResolvedStop {
    pub color: Rgba<u8>,
    pub position: f32,
}

#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub angle_deg: f32,
    pub stops: Vec<ColorStop>,
}

impl LinearGradient {
    pub fn parse(gradient_str: &str) -> Option<Self> {
        let inner = gradient_str.strip_prefix("linear-gradient(")?.strip_suffix(")")?;

        let parts = Self::split_gradient_parts(inner);
        if parts.is_empty() {
            return None;
        }

        let (angle_deg, color_start_idx) = if parts[0].contains("deg")
            || parts[0].contains("grad")
            || parts[0].contains("turn")
            || parts[0].contains("rad")
            || parts[0].starts_with("to ")
            || parts[0].parse::<f32>().is_ok()
        {
            let angle = Self::parse_angle(&parts[0])?;
            (angle, 1)
        } else {
            (180.0, 0)
        };

        let stops: Vec<ColorStop> =
            parts[color_start_idx..].iter().filter_map(|s| Self::parse_color_stop(s)).collect();

        if stops.is_empty() {
            return None;
        }

        Some(LinearGradient { angle_deg, stops })
    }

    fn split_gradient_parts(s: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut paren_depth = 0;

        for ch in s.chars() {
            match ch {
                '(' => {
                    paren_depth += 1;
                    current.push(ch);
                }
                ')' => {
                    paren_depth -= 1;
                    current.push(ch);
                }
                ',' if paren_depth == 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        if !current.is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
    }

    fn parse_angle(angle_str: &str) -> Option<f32> {
        let angle_str = angle_str.trim();

        if angle_str.starts_with("to ") {
            return Some(match angle_str {
                "to top" => 0.0,
                "to right" => 90.0,
                "to bottom" => 180.0,
                "to left" => 270.0,
                "to top right" | "to right top" => 45.0,
                "to bottom right" | "to right bottom" => 135.0,
                "to bottom left" | "to left bottom" => 225.0,
                "to top left" | "to left top" => 315.0,
                _ => return None,
            });
        }

        if let Some(deg_str) = angle_str.strip_suffix("deg") {
            deg_str.trim().parse::<f32>().ok().map(|v| v.rem_euclid(360.0))
        } else if let Some(grad_str) = angle_str.strip_suffix("grad") {
            grad_str.trim().parse::<f32>().ok().map(|v| (v / 400.0 * 360.0).rem_euclid(360.0))
        } else if let Some(turn_str) = angle_str.strip_suffix("turn") {
            turn_str.trim().parse::<f32>().ok().map(|v| (v * 360.0).rem_euclid(360.0))
        } else if let Some(rad_str) = angle_str.strip_suffix("rad") {
            rad_str.trim().parse::<f32>().ok().map(|v| v.to_degrees().rem_euclid(360.0))
        } else {
            angle_str.parse::<f32>().ok().map(|v| v.rem_euclid(360.0))
        }
    }

    fn parse_color_stop(stop_str: &str) -> Option<ColorStop> {
        let parts: Vec<&str> = stop_str.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let color = Self::parse_color(parts[0])?;

        let position = if parts.len() > 1 { Self::parse_position(parts[1]) } else { None };

        Some(ColorStop { color, position })
    }

    fn parse_position(pos_str: &str) -> Option<StopPosition> {
        if let Some(pct_str) = pos_str.strip_suffix('%') {
            pct_str.parse::<f32>().ok().map(StopPosition::Percentage)
        } else if let Some(px_str) = pos_str.strip_suffix("px") {
            px_str.parse::<f32>().ok().map(StopPosition::Px)
        } else {
            pos_str.parse::<f32>().ok().map(|v| StopPosition::Normalized(v.clamp(0.0, 1.0)))
        }
    }

    fn parse_color(color_str: &str) -> Option<Rgba<u8>> {
        let color_str = color_str.trim();

        let rgba = match color_str {
            "black" => [0, 0, 0, 255],
            "white" => [255, 255, 255, 255],
            "red" => [255, 0, 0, 255],
            "green" => [0, 255, 0, 255],
            "blue" => [0, 0, 255, 255],
            "yellow" => [255, 255, 0, 255],
            "cyan" => [0, 255, 255, 255],
            "magenta" => [255, 0, 255, 255],
            "gray" | "grey" => [128, 128, 128, 255],
            "transparent" => [0, 0, 0, 0],
            "orange" => [255, 165, 0, 255],
            "purple" => [128, 0, 128, 255],
            "pink" => [255, 192, 203, 255],
            "brown" => [165, 42, 42, 255],
            "lime" => [0, 255, 0, 255],
            "indigo" => [75, 0, 130, 255],
            "violet" => [238, 130, 238, 255],
            "navy" => [0, 0, 128, 255],
            "teal" => [0, 128, 128, 255],
            "olive" => [128, 128, 0, 255],
            "maroon" => [128, 0, 0, 255],
            "silver" => [192, 192, 192, 255],
            _ => {
                if color_str.starts_with('#') {
                    let hex = color_str.trim_start_matches('#');
                    if hex.len() == 6 {
                        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                        [r, g, b, 255]
                    } else if hex.len() == 3 {
                        let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                        let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                        let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                        [r, g, b, 255]
                    } else if hex.len() == 8 {
                        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                        [r, g, b, a]
                    } else {
                        return None;
                    }
                } else if color_str.starts_with("rgb(") {
                    let inner = color_str.strip_prefix("rgb(")?.strip_suffix(")")?;
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 3 {
                        let r = parts[0].parse().ok()?;
                        let g = parts[1].parse().ok()?;
                        let b = parts[2].parse().ok()?;
                        [r, g, b, 255]
                    } else {
                        return None;
                    }
                } else if color_str.starts_with("rgba(") {
                    let inner = color_str.strip_prefix("rgba(")?.strip_suffix(")")?;
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 4 {
                        let r = parts[0].parse().ok()?;
                        let g = parts[1].parse().ok()?;
                        let b = parts[2].parse().ok()?;
                        let a = (parts[3].parse::<f32>().ok()? * 255.0) as u8;
                        [r, g, b, a]
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
        };

        Some(Rgba(rgba))
    }

    fn resolve_stops(&self, axis_length: f32) -> Vec<ResolvedStop> {
        if self.stops.is_empty() {
            return vec![];
        }

        if self.stops.len() == 1 {
            return vec![ResolvedStop { color: self.stops[0].color, position: axis_length / 2.0 }];
        }

        let mut resolved: Vec<ResolvedStop> = Vec::with_capacity(self.stops.len());

        for stop in &self.stops {
            let position = stop.position.map(|p| p.to_px(axis_length)).unwrap_or(-1.0);
            resolved.push(ResolvedStop { color: stop.color, position });
        }

        if resolved[0].position < 0.0 {
            resolved[0].position = 0.0;
        }
        let last_idx = resolved.len() - 1;
        if resolved[last_idx].position < 0.0 {
            resolved[last_idx].position = axis_length;
        }

        let mut i = 1;
        while i < resolved.len() - 1 {
            if resolved[i].position >= 0.0 {
                i += 1;
                continue;
            }

            let start_idx = i - 1;
            let mut end_idx = i;
            while end_idx < resolved.len() && resolved[end_idx].position < 0.0 {
                end_idx += 1;
            }

            let start_pos = resolved[start_idx].position;
            let end_pos = resolved[end_idx].position;
            let segments = (end_idx - start_idx) as f32;

            for (offset, stop) in resolved[(start_idx + 1)..end_idx].iter_mut().enumerate() {
                stop.position = start_pos + (end_pos - start_pos) * (offset + 1) as f32 / segments;
            }

            i = end_idx + 1;
        }

        for i in 1..resolved.len() {
            if resolved[i].position < resolved[i - 1].position {
                resolved[i].position = resolved[i - 1].position;
            }
        }

        resolved
    }

    pub fn color_at(&self, position: f32, axis_length: f32) -> Rgba<u8> {
        let resolved = self.resolve_stops(axis_length);

        if resolved.is_empty() {
            return Rgba([0, 0, 0, 0]);
        }
        if resolved.len() == 1 {
            return resolved[0].color;
        }

        let position_px = position * axis_length;

        let mut left_idx = 0;
        let mut right_idx = resolved.len() - 1;

        for (i, stop) in resolved.iter().enumerate() {
            if stop.position <= position_px {
                left_idx = i;
            }
            if stop.position >= position_px && i > left_idx {
                right_idx = i;
                break;
            }
        }

        if left_idx == right_idx {
            return resolved[left_idx].color;
        }

        let left = &resolved[left_idx];
        let right = &resolved[right_idx];

        let denom = right.position - left.position;
        let t = if denom.abs() < f32::EPSILON {
            0.0
        } else {
            ((position_px - left.position) / denom).clamp(0.0, 1.0)
        };

        let c1 = left.color;
        let c2 = right.color;

        Rgba([
            (c1[0] as f32 * (1.0 - t) + c2[0] as f32 * t) as u8,
            (c1[1] as f32 * (1.0 - t) + c2[1] as f32 * t) as u8,
            (c1[2] as f32 * (1.0 - t) + c2[2] as f32 * t) as u8,
            (c1[3] as f32 * (1.0 - t) + c2[3] as f32 * t) as u8,
        ])
    }

    pub fn calculate_params(&self, width: f32, height: f32) -> GradientParams {
        let angle_rad = self.angle_deg.to_radians();
        let dir_x = angle_rad.sin();
        let dir_y = -angle_rad.cos();

        let cx = width / 2.0;
        let cy = height / 2.0;
        let max_extent = ((width * dir_x.abs()) + (height * dir_y.abs())) / 2.0;
        let axis_length = 2.0 * max_extent;

        GradientParams { dir_x, dir_y, cx, cy, max_extent, axis_length }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GradientParams {
    pub dir_x: f32,
    pub dir_y: f32,
    pub cx: f32,
    pub cy: f32,
    pub max_extent: f32,
    pub axis_length: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_gradient() {
        let grad = LinearGradient::parse("linear-gradient(red, blue)").unwrap();
        assert_eq!(grad.angle_deg, 180.0);
        assert_eq!(grad.stops.len(), 2);
    }

    #[test]
    fn test_parse_with_angle() {
        let grad = LinearGradient::parse("linear-gradient(45deg, red, blue)").unwrap();
        assert_eq!(grad.angle_deg, 45.0);
    }

    #[test]
    fn test_parse_with_direction() {
        let grad = LinearGradient::parse("linear-gradient(to right, red, blue)").unwrap();
        assert_eq!(grad.angle_deg, 90.0);
    }

    #[test]
    fn test_parse_hex_colors() {
        let grad = LinearGradient::parse("linear-gradient(#ff0000, #0000ff)").unwrap();
        assert_eq!(grad.stops[0].color, Rgba([255, 0, 0, 255]));
        assert_eq!(grad.stops[1].color, Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn test_parse_with_positions() {
        let grad = LinearGradient::parse("linear-gradient(to right, red 0%, blue 100%)").unwrap();
        assert_eq!(grad.angle_deg, 90.0);
        assert!(matches!(grad.stops[0].position, Some(StopPosition::Percentage(p)) if p == 0.0));
        assert!(matches!(grad.stops[1].position, Some(StopPosition::Percentage(p)) if p == 100.0));
    }

    #[test]
    fn test_parse_angle_units() {
        let grad = LinearGradient::parse("linear-gradient(90deg, red, blue)").unwrap();
        assert_eq!(grad.angle_deg, 90.0);

        let grad = LinearGradient::parse("linear-gradient(200grad, red, blue)").unwrap();
        assert_eq!(grad.angle_deg, 180.0);

        let grad = LinearGradient::parse("linear-gradient(0.5turn, red, blue)").unwrap();
        assert_eq!(grad.angle_deg, 180.0);
    }

    #[test]
    fn test_color_interpolation() {
        let grad = LinearGradient {
            angle_deg: 0.0,
            stops: vec![
                ColorStop {
                    color: Rgba([255, 0, 0, 255]),
                    position: Some(StopPosition::Percentage(0.0)),
                },
                ColorStop {
                    color: Rgba([0, 0, 255, 255]),
                    position: Some(StopPosition::Percentage(100.0)),
                },
            ],
        };

        let start = grad.color_at(0.0, 100.0);
        assert_eq!(start, Rgba([255, 0, 0, 255]));

        let end = grad.color_at(1.0, 100.0);
        assert_eq!(end, Rgba([0, 0, 255, 255]));

        let middle = grad.color_at(0.5, 100.0);
        assert!(middle[0] >= 127 && middle[0] <= 128);
        assert_eq!(middle[1], 0);
        assert!(middle[2] >= 127 && middle[2] <= 128);
        assert_eq!(middle[3], 255);
    }

    #[test]
    fn test_resolve_stops_undefined() {
        let grad = LinearGradient {
            angle_deg: 0.0,
            stops: vec![
                ColorStop { color: Rgba([255, 0, 0, 255]), position: None },
                ColorStop { color: Rgba([0, 255, 0, 255]), position: None },
                ColorStop { color: Rgba([0, 0, 255, 255]), position: None },
            ],
        };

        let resolved = grad.resolve_stops(100.0);
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].position, 0.0);
        assert_eq!(resolved[1].position, 50.0);
        assert_eq!(resolved[2].position, 100.0);
    }

    #[test]
    fn test_parse_rgba() {
        let grad = LinearGradient::parse("linear-gradient(rgba(255,0,0,0.5), blue)").unwrap();
        assert_eq!(grad.stops[0].color, Rgba([255, 0, 0, 127]));
        assert_eq!(grad.stops[1].color, Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn test_parse_8_digit_hex() {
        let grad = LinearGradient::parse("linear-gradient(#ff000080, blue)").unwrap();
        assert_eq!(grad.stops[0].color, Rgba([255, 0, 0, 128]));
    }

    #[test]
    fn test_parse_px_positions() {
        let grad = LinearGradient::parse("linear-gradient(to right, red 10px, blue 90px)").unwrap();
        assert_eq!(grad.angle_deg, 90.0);
        assert!(matches!(grad.stops[0].position, Some(StopPosition::Px(p)) if p == 10.0));
        assert!(matches!(grad.stops[1].position, Some(StopPosition::Px(p)) if p == 90.0));
    }

    #[test]
    fn test_resolve_stops_with_px() {
        let grad = LinearGradient {
            angle_deg: 0.0,
            stops: vec![
                ColorStop { color: Rgba([255, 0, 0, 255]), position: Some(StopPosition::Px(0.0)) },
                ColorStop { color: Rgba([0, 255, 0, 255]), position: Some(StopPosition::Px(50.0)) },
                ColorStop {
                    color: Rgba([0, 0, 255, 255]),
                    position: Some(StopPosition::Px(100.0)),
                },
            ],
        };

        let resolved = grad.resolve_stops(100.0);
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].position, 0.0);
        assert_eq!(resolved[1].position, 50.0);
        assert_eq!(resolved[2].position, 100.0);
    }

    #[test]
    fn test_resolve_stops_mixed_units() {
        let grad = LinearGradient {
            angle_deg: 0.0,
            stops: vec![
                ColorStop {
                    color: Rgba([255, 0, 0, 255]),
                    position: Some(StopPosition::Percentage(0.0)),
                },
                ColorStop { color: Rgba([0, 255, 0, 255]), position: Some(StopPosition::Px(50.0)) },
                ColorStop {
                    color: Rgba([0, 0, 255, 255]),
                    position: Some(StopPosition::Percentage(100.0)),
                },
            ],
        };

        let resolved = grad.resolve_stops(100.0);
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].position, 0.0);
        assert_eq!(resolved[1].position, 50.0);
        assert_eq!(resolved[2].position, 100.0);
    }
}
