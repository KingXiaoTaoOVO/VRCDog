from PIL import Image
import os

img_path = r"src\assets\dog.jpg"
img = Image.open(img_path)

def create_nsis_bmp(target_width, target_height, output_path):
    img_ratio = img.width / img.height
    target_ratio = target_width / target_height

    if img_ratio > target_ratio:
        new_width = int(target_ratio * img.height)
        offset = (img.width - new_width) // 2
        crop_img = img.crop((offset, 0, offset + new_width, img.height))
    else:
        new_height = int(img.width / target_ratio)
        offset = (img.height - new_height) // 2
        crop_img = img.crop((0, offset, img.width, offset + new_height))

    res = crop_img.resize((target_width, target_height), Image.Resampling.LANCZOS)
    res.save(output_path, format="BMP")

create_nsis_bmp(164, 314, r"src-tauri\icons\sidebar.bmp")
create_nsis_bmp(150, 57, r"src-tauri\icons\header.bmp")
print("NSIS BMPs created successfully")
