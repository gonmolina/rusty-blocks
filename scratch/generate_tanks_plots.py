import csv
import os

def generate_svg(csv_path, output_paths, title, x_col, y_cols, y_labels, y_unit, colors, stroke_widths=None, stroke_dashes=None):
    with open(csv_path, 'r') as f:
        reader = csv.reader(f)
        header = next(reader)
        data = []
        for row in reader:
            if not row:
                continue
            data.append([float(x) for x in row])

    x_idx = header.index(x_col)
    y_indices = [header.index(col) for col in y_cols]

    x_data = [row[x_idx] for row in data]
    y_series = [[row[idx] for row in data] for idx in y_indices]

    width = 800
    height = 480
    m_left = 80
    m_right = 40
    m_top = 60
    m_bottom = 80

    plot_w = width - m_left - m_right
    plot_h = height - m_top - m_bottom

    x_min, x_max = min(x_data), max(x_data)
    y_flat = [val for s in y_series for val in s]
    y_min, y_max = min(y_flat), max(y_flat)
    y_range = (y_max - y_min) if (y_max - y_min) > 0 else 1.0
    y_min -= y_range * 0.05
    y_max += y_range * 0.05

    def to_x(val):
        return m_left + (val - x_min) / (x_max - x_min) * plot_w if (x_max - x_min) > 0 else m_left

    def to_y(val):
        return m_top + plot_h - (val - y_min) / (y_max - y_min) * plot_h if (y_max - y_min) > 0 else m_top + plot_h

    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="100%" height="100%" style="background:#f8fafc; font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica,Arial,sans-serif; border-radius:12px; border: 1px solid #e2e8f0;">')
    
    svg.append(f'<text x="{width / 2}" y="35" text-anchor="middle" font-size="18" font-weight="bold" fill="#0f172a">{title}</text>')

    # Grid & Ticks X (Time in minutes)
    n_ticks_x = 6
    for i in range(n_ticks_x):
        ratio = i / (n_ticks_x - 1)
        val = x_min + ratio * (x_max - x_min)
        x_pos = to_x(val)
        svg.append(f'<line x1="{x_pos}" y1="{m_top}" x2="{x_pos}" y2="{m_top + plot_h}" stroke="#e2e8f0" stroke-dasharray="3,3" />')
        svg.append(f'<text x="{x_pos}" y="{m_top + plot_h + 20}" text-anchor="middle" font-size="11" fill="#64748b">{val:.0f} s</text>')

    # Grid & Ticks Y
    n_ticks_y = 6
    for i in range(n_ticks_y):
        ratio = i / (n_ticks_y - 1)
        val = y_min + ratio * (y_max - y_min)
        y_pos = to_y(val)
        svg.append(f'<line x1="{m_left}" y1="{y_pos}" x2="{m_left + plot_w}" y2="{y_pos}" stroke="#e2e8f0" />')
        svg.append(f'<text x="{m_left - 10}" y="{y_pos + 4}" text-anchor="end" font-size="11" fill="#64748b">{val:.3f} {y_unit}</text>')

    # Plot lines
    step_ds = max(1, len(x_data) // 400)
    
    for s_idx, s_data in enumerate(y_series):
        points = []
        for j in range(0, len(x_data), step_ds):
            x_val = x_data[j]
            y_val = s_data[j]
            points.append(f"{to_x(x_val):.1f},{to_y(y_val):.1f}")
        if (len(x_data) - 1) % step_ds != 0:
            points.append(f"{to_x(x_data[-1]):.1f},{to_y(s_data[-1]):.1f}")
            
        pts_str = " ".join(points)
        color = colors[s_idx % len(colors)]
        w = stroke_widths[s_idx] if stroke_widths and s_idx < len(stroke_widths) else 2.0
        dash = f' stroke-dasharray="{stroke_dashes[s_idx]}"' if stroke_dashes and s_idx < len(stroke_dashes) and stroke_dashes[s_idx] else ''
        svg.append(f'<polyline fill="none" stroke="{color}" stroke-width="{w}"{dash} stroke-linecap="round" stroke-linejoin="round" points="{pts_str}" />')

    # Legend
    legend_x_start = m_left
    legend_y = height - 40
    svg.append(f'<g transform="translate({legend_x_start}, {legend_y})">')
    
    offset_x = 0
    offset_y = 0
    for s_idx, label in enumerate(y_labels):
        color = colors[s_idx % len(colors)]
        dash = stroke_dashes[s_idx] if stroke_dashes and s_idx < len(stroke_dashes) else None
        
        if dash:
            svg.append(f'<line x1="{offset_x}" y1="{offset_y - 6}" x2="{offset_x + 16}" y2="{offset_y - 6}" stroke="{color}" stroke-width="2.5" stroke-dasharray="{dash}"/>')
        else:
            svg.append(f'<rect x="{offset_x}" y="{offset_y - 10}" width="16" height="8" fill="{color}" rx="2"/>')
            
        svg.append(f'<text x="{offset_x + 22}" y="{offset_y - 3}" font-size="11" fill="#334155">{label}</text>')
        
        offset_x += len(label) * 7.0 + 40
        if offset_x > plot_w - 120:
            offset_x = 0
            offset_y += 18

    svg.append('</g>')
    svg.append('</svg>')

    for p in output_paths:
        with open(p, 'w') as f:
            f.write("\n".join(svg))
    print(f"Gráfico generado exitosamente en: {output_paths[0]}")

if __name__ == "__main__":
    csv_path = "results/dos_tanques.csv"
    art_dir = "/home/gonza/.gemini/antigravity-cli/brain/bc54441e-1fe6-4450-a517-074c2102dea2"
    root_dir = "/home/gonza/dev/CNEAProjects/rusty-blocks"
    
    os.makedirs(art_dir, exist_ok=True)

    # ── PLOT 1: NIVELES ──────────────────────────────────────────────
    generate_svg(
        csv_path=csv_path,
        output_paths=[
            os.path.join(art_dir, "tanks_levels.svg"),
            os.path.join(root_dir, "tanks_levels.svg")
        ],
        title="Dos Tanques: Evolución Dinámica de Niveles de Agua",
        x_col="t_s",
        y_cols=["level_a_m", "level_b_m"],
        y_labels=["Nivel Tanque A (m)", "Nivel Tanque B (m)"],
        y_unit="m",
        colors=["#ef4444", "#3b82f6"],
        stroke_widths=[2.5, 2.5]
    )

    # ── PLOT 2: CAUDAL ──────────────────────────────────────────────
    generate_svg(
        csv_path=csv_path,
        output_paths=[
            os.path.join(art_dir, "tanks_flow.svg"),
            os.path.join(root_dir, "tanks_flow.svg")
        ],
        title="Dos Tanques: Caudal Másico de Interconexión",
        x_col="t_s",
        y_cols=["flow_kg_s"],
        y_labels=["Caudal (kg/s)"],
        y_unit="kg/s",
        colors=["#10b981"],
        stroke_widths=[3.0]
    )
