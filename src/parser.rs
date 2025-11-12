use crate::types::*;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn load_timesheets(path: &Path) -> Result<Vec<Timesheet>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .context("无法获取文件扩展名")?;

    match ext.as_str() {
        "xdts" => load_xdts(path),
        "tdts" => load_tdts(path),
        _ => anyhow::bail!("不支持的文件格式: {}", ext),
    }
}

fn load_xdts(path: &Path) -> Result<Vec<Timesheet>> {
    let json_str = read_json_file(path)?;
    let root: XDTSRoot = serde_json::from_str(&json_str)
        .context("解析 XDTS JSON 失败")?;

    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let mut timesheets = Vec::new();
    for time_table in root.time_tables {
        let name = format!("{}->{}", filename, time_table.name);
        let timesheet = parse_xdts_timetable(name, time_table)?;
        timesheets.push(timesheet);
    }

    Ok(timesheets)
}

fn load_tdts(path: &Path) -> Result<Vec<Timesheet>> {
    let json_str = read_json_file(path)?;
    let root: TDTSRoot = serde_json::from_str(&json_str)
        .context("解析 TDTS JSON 失败")?;

    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let mut timesheets = Vec::new();
    for time_sheet in root.time_sheets {
        let cut_name = &time_sheet.header.cut;
        for time_table in time_sheet.time_tables {
            if !time_table.fields.is_empty() {
                let name = format!("{}->{}->{}",
                    filename, cut_name, time_table.name);
                let timesheet = parse_tdts_timetable(name, time_table)?;
                timesheets.push(timesheet);
            }
        }
    }

    Ok(timesheets)
}

fn read_json_file(path: &Path) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("无法打开文件: {}", path.display()))?;
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader
        .lines()
        .skip(1) // 跳过第一行注释
        .collect::<Result<_, _>>()
        .context("读取文件失败")?;

    Ok(lines.join("\n"))
}

fn parse_xdts_timetable(name: String, time_table: TimeTable) -> Result<Timesheet> {
    let frame_count = time_table.duration;

    if time_table.fields.is_empty() {
        return Ok(Timesheet {
            name,
            frame_count,
            layers: Vec::new(),
        });
    }

    let field = &time_table.fields[0];
    let field_id = field.field_id;

    // 查找对应的名称列表
    let names = time_table
        .time_table_headers
        .iter()
        .find(|h| h.field_id == field_id)
        .map(|h| &h.names);

    let mut layers = Vec::new();

    if let Some(names) = names {
        for track in &field.tracks {
            let layer_name = names
                .get(track.track_no)
                .cloned()
                .unwrap_or_else(|| format!("Layer {}", track.track_no));

            let mut frames = Vec::new();
            for frame_data in &track.frames {
                if let Some(value) = frame_data.data.get(0).and_then(|d| d.values.get(0)) {
                    let cell = parse_xdts_cell_value(value);
                    if let Some(cell) = cell {
                        frames.push(Frame {
                            frame: frame_data.frame,
                            cell,
                        });
                    }
                }
            }

            // 优化关键帧
            optimize_frames(&mut frames);

            layers.push(Layer {
                name: layer_name,
                frames,
            });
        }
    }

    Ok(Timesheet {
        name,
        frame_count,
        layers,
    })
}

fn parse_tdts_timetable(name: String, time_table: TimeTable) -> Result<Timesheet> {
    let frame_count = time_table.duration;

    // 查找 fieldId = 4 的 field
    let field = time_table
        .fields
        .iter()
        .find(|f| f.field_id == 4);

    // 查找对应的名称列表
    let names = time_table
        .time_table_headers
        .iter()
        .find(|h| h.field_id == 4)
        .map(|h| &h.names);

    let mut layers = Vec::new();

    if let (Some(field), Some(names)) = (field, names) {
        for track in &field.tracks {
            let layer_name = names
                .get(track.track_no)
                .cloned()
                .unwrap_or_else(|| format!("Layer {}", track.track_no));

            let mut frames = Vec::new();
            for frame_data in &track.frames {
                if let Some(value) = frame_data.data.get(0).and_then(|d| d.values.get(0)) {
                    let cell = parse_tdts_cell_value(value);
                    frames.push(Frame {
                        frame: frame_data.frame,
                        cell,
                    });
                }
            }
            optimize_frames(&mut frames);

            layers.push(Layer {
                name: layer_name,
                frames,
            });
        }
    }

    Ok(Timesheet {
        name,
        frame_count,
        layers,
    })
}

fn parse_xdts_cell_value(value: &str) -> Option<u16> {
    if value == "SYMBOL_NULL_CELL" {
        return Some(0);
    }

    if matches!(value, "SYMBOL_TICK_1" | "SYMBOL_TICK_2" | "SYMBOL_HYPHEN") {
        return None; // 跳过这些特殊符号
    }

    // 提取末尾的数字
    let digits: String = value.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    let digits: String = digits.chars().rev().collect();

    if !digits.is_empty() {
        digits.parse().ok()
    } else {
        None
    }
}

fn parse_tdts_cell_value(value: &str) -> u16 {
    if value == "SYMBOL_NULL_CELL" {
        return 0;
    }

    value.parse().unwrap_or(0)
}

fn optimize_frames(frames: &mut Vec<Frame>) {
    if frames.is_empty() {
        return;
    }

    // 确保第一帧从 0 开始
    if frames[0].frame != 0 {
        frames.insert(0, Frame { frame: 0, cell: 0 });
    }

    // 移除连续相同的 cell 值
    let mut i = frames.len() - 1;
    while i > 0 {
        if frames[i].cell == frames[i - 1].cell {
            frames.remove(i);
        }
        if i > 0 {
            i -= 1;
        }
    }
}
