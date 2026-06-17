import xml.etree.ElementTree as ET

tree = ET.parse("blockdiagram.xml")
root = tree.getroot()

def find_block(elem, name):
    if elem.tag == "Block" and elem.attrib.get("Name") == name:
        return elem
    for child in elem:
        res = find_block(child, name)
        if res is not None:
            return res
    return None

b = find_block(root, "Thermohydraulic phw")
if b is not None:
    for child in b:
        if child.tag == "System":
            for gc in child:
                print(f"  System Child: {gc.tag}, attrib: {gc.attrib}")
                # If there are nested Blocks, print them
                if gc.tag == "Block":
                    print(f"    Block Type: {gc.attrib.get('BlockType')}, Name: {gc.attrib.get('Name')}")
