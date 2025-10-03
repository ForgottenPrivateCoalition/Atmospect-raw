import tkinter as tk
from tkinter import filedialog
import os
import gzip

def convert_to_svgz(file_paths, output_folder=None):
    if not file_paths:
        print("Файлы не выбраны")
        return

    if not output_folder:
        output_folder = os.path.dirname(file_paths[0])
    os.makedirs(output_folder, exist_ok=True)

    for file_path in file_paths:
        filename = os.path.basename(file_path)
        name, _ = os.path.splitext(filename)
        new_path = os.path.join(output_folder, f"{name}.svgz")

        try:
            with open(file_path, "rb") as f_in:
                with gzip.open(new_path, "wb") as f_out:
                    f_out.writelines(f_in)
            print(f"Готово! {filename} → {new_path}")
        except Exception as e:
            print(f"Ошибка при обработке {filename}: {e}")

def main():
    root = tk.Tk()
    root.withdraw()

    file_paths = filedialog.askopenfilenames(
        title="Выберите SVG файлы для конвертации в SVGZ",
        filetypes=[("SVG files", "*.svg")]
    )

    convert_to_svgz(file_paths)

if __name__ == "__main__":
    main()
