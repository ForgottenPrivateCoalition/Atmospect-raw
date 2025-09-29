# Apache License Version 2.0, January 2004
# JPGMetaCreationTool (C) 2025 Forgotten Private Coalition

import piexif
from tkinter import filedialog, Tk
import os

def add_metadata_jpg(file_path):
    exif_dict = piexif.load(file_path)

    exif_dict["0th"][piexif.ImageIFD.Artist] = "Forgotten Private Coalition"
    exif_dict["0th"][piexif.ImageIFD.Copyright] = "Atmospect Launcher (C) 2025 Forgotten Private Coalition CC BY-NC 4.0 License"

    new_path = os.path.splitext(file_path)[0] + "_meta.jpg"
    exif_bytes = piexif.dump(exif_dict)
    piexif.insert(exif_bytes, file_path, new_path)

def main():
    root = Tk()
    root.withdraw()
    file_path = filedialog.askopenfilename(
        title="Выберите JPG файл",
        filetypes=[("JPEG files", "*.jpg;*.jpeg")]
    )
    if file_path:
        add_metadata_jpg(file_path)

if __name__ == "__main__":
    main()
