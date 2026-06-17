import xml.etree.ElementTree as ET

tree = ET.parse("blockdiagram.xml")
root = tree.getroot()

block_names = set()

def collect_names(elem):
    if elem.tag == "Block":
        b_name = elem.attrib.get("Name", "")
        if b_name:
            block_names.add(b_name)
    for child in elem:
        collect_names(child)

collect_names(root)
print("All block names in SLX:")
for name in sorted(list(block_names)):
    print(f"  - {name}")
