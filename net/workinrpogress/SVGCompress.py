import re
import tkinter as tk
from tkinter import filedialog
import os

def round_number(match):
    num = float(match.group())
    rounded = round(num, 1)
    if abs(rounded - round(rounded)) < 1e-6:
        return str(int(round(rounded)))
    return str(rounded)

def compress_svg(svg_text: str) -> str:
    svg_text = re.sub(r'<\?xml.*?\?>', '', svg_text)

    def process_svg_tag(match):
        tag = match.group()
        tag = re.sub(r'\s(width|height)="[^"]*"', '', tag)
        tag = re.sub(r'\s(version|id|enable-background)="[^"]*"', '', tag)
        tag = re.sub(r'\s(x|y)="0(px)?"', '', tag)
        return tag

    svg_text = re.sub(r'<svg[^>]*>', process_svg_tag, svg_text, count=1)
    svg_text = re.sub(r'-?\d+\.\d+|-?\d+', round_number, svg_text)
    svg_text = re.sub(r'\sopacity="1"', '', svg_text)
    svg_text = re.sub(r'\sstroke="none"', '', svg_text)

    def clean_style(match):
        style_content = match.group(1).strip()
        if style_content == '' or style_content == 'stroke: rgb(0, 0, 0);':
            return ''
        return f'style="{style_content}"'

    svg_text = re.sub(r'style="([^"]*)"', clean_style, svg_text)
    svg_text = re.sub(r'\n\s*\n', '\n', svg_text)

    return svg_text.strip()

def main():
    root = tk.Tk()
    root.withdraw()

    file_paths = filedialog.askopenfilenames(
        title="Выберите SVG файлы",
        filetypes=[("SVG files", "*.svg")]
    )

    if not file_paths:
        print("Файлы не выбраны")
        return

    first_file_dir = os.path.dirname(file_paths[0])
    output_folder = os.path.join(first_file_dir, "CompressedSVG")

    try:
        os.makedirs(output_folder, exist_ok=True)
    except PermissionError:
        # fallback на рабочий стол
        desktop = os.path.join(os.path.expanduser("~"), "Desktop")
        output_folder = os.path.join(desktop, "CompressedSVG")
        os.makedirs(output_folder, exist_ok=True)
        print(f"Нет прав создать папку рядом с файлами, используем Desktop: {output_folder}")

    for file_path in file_paths:
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()

        compressed = compress_svg(content)

        filename = os.path.basename(file_path)
        new_path = os.path.join(output_folder, filename)
        with open(new_path, "w", encoding="utf-8") as f:
            f.write(compressed)

        print(f"Готово! Сжатый файл сохранён как: {new_path}")

if __name__ == "__main__":
    main()
