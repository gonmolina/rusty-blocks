import xml.etree.ElementTree as ET

tree = ET.parse("blockdiagram.xml")
root = tree.getroot()

def find_scripts(elem):
    # Search for script or code parameters
    if elem.tag == "P" and elem.attrib.get("Name") in ["script", "code", "MATLABCode", "Display"]:
        val = elem.text
        if val and len(val.strip()) > 10:
            print(f"Found code parameter '{elem.attrib.get('Name')}':")
            print(val[:500])
            print("-" * 40)
    for child in elem:
        find_scripts(child)

find_scripts(root)
