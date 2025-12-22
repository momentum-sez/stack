import pathlib, re, yaml
from lxml import etree

REPO = pathlib.Path(__file__).resolve().parents[1]

MUST_RE = re.compile(r"\b(MUST|SHALL)\b")

def load_yaml(p): return yaml.safe_load(p.read_text(encoding="utf-8"))

def extract_must_clause_eids(xml_path: pathlib.Path):
    doc = etree.parse(str(xml_path))
    # Look at paragraphs with eId and text containing MUST/SHALL
    eids = []
    for el in doc.xpath('//*[@eId]'):
        text = " ".join([t.strip() for t in el.xpath(".//text()") if str(t).strip()])
        if MUST_RE.search(text):
            eids.append(el.get("eId"))
    return eids

def map_covers_eid(map_data, eid: str) -> bool:
    if not isinstance(map_data, list):
        return False
    for entry in map_data:
        for lr in entry.get("legal_refs", []) or []:
            if isinstance(lr, dict) and lr.get("eId") == eid:
                return True
            if isinstance(lr, str) and eid in lr:
                return True
    return False

def test_every_must_clause_is_mapped_when_map_exists_or_required():
    # Rule: if a module has MUST/SHALL clauses in src/akn/*.xml, it MUST have src/policy-to-code/map.yaml
    # and that map MUST reference each MUST/SHALL clause by eId (best effort).
    for mod in REPO.glob("modules/**/module.yaml"):
        mod_dir = mod.parent
        akn_dir = mod_dir / "src" / "akn"
        if not akn_dir.exists():
            continue
        must_eids = []
        for xml in akn_dir.rglob("*.xml"):
            must_eids.extend(extract_must_clause_eids(xml))
        if not must_eids:
            continue
        map_path = mod_dir / "src" / "policy-to-code" / "map.yaml"
        assert map_path.exists(), f"Module {mod_dir} has MUST/SHALL clauses but no policy-to-code map"
        map_data = load_yaml(map_path)
        missing = [eid for eid in must_eids if not map_covers_eid(map_data, eid)]
        assert not missing, f"Module {mod_dir} missing policy-to-code entries for eIds: {missing}"
