use crate::types::*;
use anyhow::{Context, Result};
use encoding_rs::SHIFT_JIS;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn save_sts(timesheet: &Timesheet, output_path: &Path, verbose: bool) -> Result<()> {
    let layer_count = timesheet.layers.len();
    let frame_count = timesheet.frame_count as usize;

    if verbose {
        println!("\n正在转换: {}", timesheet.name);
        println!("  层数: {}", layer_count);
        println!("  帧数: {}", frame_count);
    }

    if layer_count > 255 {
        anyhow::bail!("层数过多: {}, 最大支持 255 层", layer_count);
    }

    if frame_count > 65535 {
        anyhow::bail!("帧数过多: {}, 最大支持 65535 帧", frame_count);
    }

    // 展开所有层的帧数据
    let mut all_layers_cells: Vec<Vec<u16>> = Vec::new();
    for (idx, layer) in timesheet.layers.iter().enumerate() {
        let cells = expand_frames(&layer.frames, frame_count);
        all_layers_cells.push(cells);

        if verbose {
            let unique_cells: std::collections::HashSet<_> =
                all_layers_cells[idx].iter().collect();
            let keyframe_count = layer.frames.len();
            println!(
                "  第{}层 '{}': {}个关键帧, {}个唯一cell值",
                idx + 1,
                layer.name,
                keyframe_count,
                unique_cells.len()
            );
        }
    }

    // 写入 STS 文件
    let mut file = File::create(output_path)
        .with_context(|| format!("无法创建文件: {}", output_path.display()))?;

    // === 文件头 (23 bytes) ===

    // STS 标识符
    file.write_all(&[0x11])?;

    // 固定字符串 "ShiraheiTimeSheet"
    file.write_all(b"ShiraheiTimeSheet")?;

    // 层数 (1 byte)
    file.write_all(&[layer_count as u8])?;

    // 帧数 (2 bytes, little-endian)
    file.write_all(&(frame_count as u16).to_le_bytes())?;

    // 填充 (2 bytes)
    file.write_all(&[0x00, 0x00])?;

    // === 帧数据区 (layer_count × frame_count × 2 bytes) ===

    for (layer_idx, cells) in all_layers_cells.iter().enumerate() {
        for (frame_idx, &cell) in cells.iter().enumerate() {
            if cell > 65535 {
                anyhow::bail!(
                    "Cell值超出范围: {} (层{}, 帧{})",
                    cell,
                    layer_idx + 1,
                    frame_idx
                );
            }
            file.write_all(&cell.to_le_bytes())?;
        }
    }

    // === 层名称区 ===

    for layer in &timesheet.layers {
        let name = &layer.name;

        // 编码为 Shift-JIS
        let (name_bytes, _, had_errors) = SHIFT_JIS.encode(name);

        if had_errors {
            eprintln!("  警告: 层名称 '{}' 包含无法编码为Shift-JIS的字符", name);
        }

        let name_bytes = if name_bytes.len() > 255 {
            eprintln!("  警告: 层名称过长，截断为255字节: '{}'", name);
            &name_bytes[..255]
        } else {
            &name_bytes
        };

        // 写入: [1字节长度][N字节名称]
        file.write_all(&[name_bytes.len() as u8])?;
        file.write_all(name_bytes)?;
    }

    if verbose {
        let actual_size = file.metadata()?.len();
        println!("\n文件已生成: {}", output_path.display());
        println!("  实际大小: {} 字节", actual_size);
    }

    Ok(())
}

/// 将关键帧列表展开为完整的帧序列
fn expand_frames(frames: &[Frame], frame_count: usize) -> Vec<u16> {
    let mut cells = vec![0u16; frame_count];

    if frames.is_empty() {
        return cells;
    }

    for i in 0..frames.len() {
        let start_frame = frames[i].frame as usize;
        let cell_value = frames[i].cell;

        // 确定结束帧
        let end_frame = if i + 1 < frames.len() {
            frames[i + 1].frame as usize
        } else {
            frame_count
        };

        // 填充该区间的所有帧
        for frame_no in start_frame..end_frame.min(frame_count) {
            cells[frame_no] = cell_value;
        }
    }

    cells
}
