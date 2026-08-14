import os
from PIL import Image, ImageDraw

def create_termcmd_icon(size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    pad = int(size * 0.08)
    radius = int(size * 0.18)
    
    bg_color = (26, 27, 38, 255)
    border_color = (65, 72, 104, 255)
    
    draw.rounded_rectangle(
        [pad, pad, size - pad, size - pad],
        radius=radius,
        fill=bg_color,
        outline=border_color,
        width=max(1, int(size * 0.02))
    )
    
    header_h = int(size * 0.22)
    draw.rounded_rectangle(
        [pad, pad, size - pad, pad + header_h],
        radius=radius,
        fill=(22, 22, 30, 255)
    )
    draw.rectangle(
        [pad, pad + radius, size - pad, pad + header_h],
        fill=(22, 22, 30, 255)
    )
    
    dot_r = max(2, int(size * 0.035))
    dot_y = pad + int(header_h * 0.5)
    dot_colors = [(247, 118, 142), (224, 175, 104), (158, 206, 106)]
    start_x = pad + int(size * 0.08)
    spacing = int(size * 0.075)
    
    for i, color in enumerate(dot_colors):
        cx = start_x + i * spacing
        draw.ellipse([cx - dot_r, dot_y - dot_r, cx + dot_r, dot_y + dot_r], fill=color)
        
    term_x = pad + int(size * 0.12)
    term_y = pad + header_h + int(size * 0.12)
    chevron_size = int(size * 0.22)
    stroke_w = max(2, int(size * 0.045))
    
    p1 = (term_x, term_y)
    p2 = (term_x + int(chevron_size * 0.6), term_y + int(chevron_size * 0.5))
    p3 = (term_x, term_y + chevron_size)
    
    prompt_color = (122, 162, 247, 255)
    draw.line([p1, p2], fill=prompt_color, width=stroke_w)
    draw.line([p2, p3], fill=prompt_color, width=stroke_w)
    
    cursor_x = term_x + chevron_size + int(size * 0.05)
    cursor_y = term_y + int(chevron_size * 0.1)
    cursor_w = int(size * 0.16)
    cursor_h = int(chevron_size * 0.8)
    
    cursor_color = (115, 218, 202, 255)
    draw.rectangle([cursor_x, cursor_y, cursor_x + cursor_w, cursor_y + cursor_h], fill=cursor_color)
    
    return img

def main():
    icons_dir = "src-tauri/icons"
    os.makedirs(icons_dir, exist_ok=True)
    
    sizes = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
    }
    
    generated_images = {}
    for filename, size in sizes.items():
        img = create_termcmd_icon(size)
        filepath = os.path.join(icons_dir, filename)
        img.save(filepath, "PNG")
        generated_images[size] = img
        print(f"Generated {filepath} ({size}x{size})")
        
    ico_path = os.path.join(icons_dir, "icon.ico")
    generated_images[512].save(
        ico_path,
        format="ICO",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    )
    print(f"Generated {ico_path}")

if __name__ == "__main__":
    main()
