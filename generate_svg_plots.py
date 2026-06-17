import csv
import os

def generate_svg(csv_path, output_path, title, x_col, y1_cols, y2_cols=None, y1_labels=None, y2_labels=None, y1_unit="", y2_unit="", colors=None):
    # Read data
    with open(csv_path, 'r') as f:
        reader = csv.reader(f)
        header = next(reader)
        data = []
        for row in reader:
            if not row:
                continue
            data.append([float(x) for x in row])

    # Find column indices
    x_idx = header.index(x_col)
    y1_indices = [header.index(col) for col in y1_cols]
    y2_indices = [header.index(col) for col in y2_cols] if y2_cols else []

    # Extract series
    x_data = [row[x_idx] for row in data]
    y1_series = [[row[idx] for row in data] for idx in y1_indices]
    y2_series = [[row[idx] for row in data] for idx in y2_indices] if y2_indices else []

    # SVG layout
    width = 800
    height = 450
    m_left = 80
    m_right = 80 if y2_cols else 40
    m_top = 60
    m_bottom = 60

    plot_w = width - m_left - m_right
    plot_h = height - m_top - m_bottom

    # Min/Max values
    x_min, x_max = min(x_data), max(x_data)
    
    y1_flat = [val for s in y1_series for val in s]
    y1_min, y1_max = min(y1_flat), max(y1_flat)
    # add margin to y limits
    y1_range = (y1_max - y1_min) if (y1_max - y1_min) > 0 else 1.0
    y1_min -= y1_range * 0.1
    y1_max += y1_range * 0.1

    if y2_cols:
        y2_flat = [val for s in y2_series for val in s]
        y2_min, y2_max = min(y2_flat), max(y2_flat)
        y2_range = (y2_max - y2_min) if (y2_max - y2_min) > 0 else 1.0
        y2_min -= y2_range * 0.1
        y2_max += y2_range * 0.1
    else:
        y2_min, y2_max = 0.0, 1.0

    # Scale helpers
    def to_x(val):
        return m_left + (val - x_min) / (x_max - x_min) * plot_w if (x_max - x_min) > 0 else m_left

    def to_y1(val):
        return m_top + plot_h - (val - y1_min) / (y1_max - y1_min) * plot_h if (y1_max - y1_min) > 0 else m_top + plot_h

    def to_y2(val):
        return m_top + plot_h - (val - y2_min) / (y2_max - y2_min) * plot_h if (y2_max - y2_min) > 0 else m_top + plot_h

    # Color palette
    if not colors:
        colors = ["#3B82F6", "#EF4444", "#10B981", "#F59E0B", "#8B5CF6"]

    # Start SVG
    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="100%" height="100%" style="background:#f8fafc; font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica,Arial,sans-serif; border-radius:12px; border: 1px solid #e2e8f0;">')
    
    # Title
    svg.append(f'<text x="{width / 2}" y="35" text-anchor="middle" font-size="18" font-weight="bold" fill="#0f172a">{title}</text>')

    # Gridlines & Axes
    # X Grid and Labels
    n_ticks_x = 5
    for i in range(n_ticks_x):
        ratio = i / (n_ticks_x - 1)
        val = x_min + ratio * (x_max - x_min)
        x_pos = to_x(val)
        svg.append(f'<line x1="{x_pos}" y1="{m_top}" x2="{x_pos}" y2="{m_top + plot_h}" stroke="#e2e8f0" stroke-dasharray="3,3" />')
        svg.append(f'<text x="{x_pos}" y="{m_top + plot_h + 20}" text-anchor="middle" font-size="11" fill="#64748b">{val:.1f} s</text>')

    # Y1 Grid and Labels (Left side)
    n_ticks_y = 5
    for i in range(n_ticks_y):
        ratio = i / (n_ticks_y - 1)
        val = y1_min + ratio * (y1_max - y1_min)
        y_pos = to_y1(val)
        svg.append(f'<line x1="{m_left}" y1="{y_pos}" x2="{m_left + plot_w}" y2="{y_pos}" stroke="#e2e8f0" />')
        svg.append(f'<text x="{m_left - 10}" y="{y_pos + 4}" text-anchor="end" font-size="11" fill="#64748b">{val:.2f} {y1_unit}</text>')

    # Y2 Labels (Right side, if exists)
    if y2_cols:
        for i in range(n_ticks_y):
            ratio = i / (n_ticks_y - 1)
            val = y2_min + ratio * (y2_max - y2_min)
            y_pos = to_y2(val)
            svg.append(f'<text x="{m_left + plot_w + 10}" y="{y_pos + 4}" text-anchor="start" font-size="11" fill="#64748b">{val:.2f} {y2_unit}</text>')

    # Draw Y1 Series
    for s_idx, s_data in enumerate(y1_series):
        points = []
        for x_val, y_val in zip(x_data, s_data):
            points.append(f"{to_x(x_val)},{to_y1(y_val)}")
        pts_str = " ".join(points)
        color = colors[s_idx % len(colors)]
        svg.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" points="{pts_str}" />')

    # Draw Y2 Series
    for s_idx, s_data in enumerate(y2_series):
        points = []
        for x_val, y_val in zip(x_data, s_data):
            points.append(f"{to_x(x_val)},{to_y2(y_val)}")
        pts_str = " ".join(points)
        color = colors[(s_idx + len(y1_series)) % len(colors)]
        svg.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.5" stroke-dasharray="5,3" stroke-linecap="round" stroke-linejoin="round" points="{pts_str}" />')

    # Legend
    legend_x = m_left
    legend_y = height - 15
    svg.append(f'<g transform="translate({legend_x}, {legend_y})">')
    offset_x = 0
    
    # Y1 Legend Items
    for s_idx, label in enumerate(y1_labels or y1_cols):
        color = colors[s_idx % len(colors)]
        svg.append(f'<rect x="{offset_x}" y="-10" width="12" height="6" fill="{color}" rx="2"/>')
        svg.append(f'<text x="{offset_x + 18}" y="-4" font-size="11" fill="#334155">{label}</text>')
        offset_x += len(label) * 7.5 + 40

    # Y2 Legend Items
    for s_idx, label in enumerate(y2_labels or y2_cols):
        color = colors[(s_idx + len(y1_series)) % len(colors)]
        svg.append(f'<line x1="{offset_x}" y1="-7" x2="{offset_x + 15}" y2="-7" stroke="{color}" stroke-width="2.5" stroke-dasharray="3,2"/>')
        svg.append(f'<text x="{offset_x + 22}" y="-4" font-size="11" fill="#334155">{label} (Der.)</text>')
        offset_x += len(label) * 7.5 + 50

    svg.append('</g>')
    svg.append('</svg>')

    # Write output file
    with open(output_path, 'w') as f:
        f.write("\n".join(svg))
    print(f"Grafico generado exitosamente en: {output_path}")

if __name__ == "__main__":
    art_dir = "/home/gonza/.gemini/antigravity-cli/brain/b87bcb8f-a0af-4eb0-b93b-2d0bfb7c4fe2"
    os.makedirs(art_dir, exist_ok=True)
    
    # 1. Plot Conveccion Natural
    generate_svg(
        csv_path="natural_convection_results.csv",
        output_path=os.path.join(art_dir, "convection_plot.svg"),
        title="Conveccion Natural - Evolucion de Caudal y Temperaturas",
        x_col="t",
        y1_cols=["w_up_in"],
        y2_cols=["t_bottom", "t_top"],
        y1_labels=["Caudal Pipe Up (kg/s)"],
        y2_labels=["Temp Header Bottom (C)", "Temp Header Top (C)"],
        y1_unit="kg/s",
        y2_unit="C",
        colors=["#3b82f6", "#06b6d4", "#f97316"]
    )
    
    # 2. Plot Bomba Centrifuga
    generate_svg(
        csv_path="pump_validation_results.csv",
        output_path=os.path.join(art_dir, "pump_plot.svg"),
        title="Bomba Centrifuga - Caudales y Presiones en Lazo Reservorio",
        x_col="t",
        y1_cols=["w_pump", "w_return"],
        y2_cols=["dp"],
        y1_labels=["Caudal Bomba (kg/s)", "Caudal Retorno (kg/s)"],
        y2_labels=["Presion dP (Pa)"],
        y1_unit="kg/s",
        y2_unit="Pa",
        colors=["#2563eb", "#10b981", "#ef4444"]
    )
