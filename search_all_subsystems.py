import xml.etree.ElementTree as ET

tree = ET.parse("blockdiagram.xml")
root = tree.getroot()

def dump_subsystems(elem, path=""):
    if elem.tag == "Block" and elem.attrib.get("BlockType") == "SubSystem":
        name = elem.attrib.get("Name", "")
        current_path = f"{path}/{name}"
        # Count nested blocks
        system_elem = elem.find("System")
        blocks_inside = []
        if system_elem is not None:
            blocks_inside = [b.attrib.get("Name", "") for b in system_elem.findall("Block")]
        print(f"Subsystem: {current_path} (SID={elem.attrib.get('SID')}) -> Blocks: {len(blocks_inside)} {blocks_inside[:5]}")
        
        # recurse into System child if it exists
        if system_elem is not None:
            for child in system_elem:
                dump_subsystems(child, current_path)
    else:
        for child in elem:
            dump_subsystems(child, path)

dump_subsystems(root)
