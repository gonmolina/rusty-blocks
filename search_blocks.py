import xml.etree.ElementTree as ET

def print_blocks(elem, depth=0):
    indent = "  " * depth
    if elem.tag == "Block":
        b_type = elem.attrib.get("BlockType", "")
        b_name = elem.attrib.get("Name", "")
        print(f"{indent}Block: Type={b_type}, Name={b_name}")
    for child in elem:
        print_blocks(child, depth + 1)

try:
    tree = ET.parse("blockdiagram.xml")
    root = tree.getroot()
    print_blocks(root)
except Exception as e:
    print(f"Error parsing XML: {e}")
