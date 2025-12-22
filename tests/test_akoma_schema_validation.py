import pathlib, pytest
from lxml import etree

REPO = pathlib.Path(__file__).resolve().parents[1]
SCHEMA_DIR = REPO / "tools" / "akoma" / "schemas"
MAIN_XSD = SCHEMA_DIR / "akomantoso30.xsd"

@pytest.mark.skipif(not MAIN_XSD.exists(), reason="Akoma schemas not fetched; run tools/msez.py fetch-akoma-schemas")
def test_all_akoma_docs_validate_against_schema():
    schema = etree.XMLSchema(etree.parse(str(MAIN_XSD)))
    for xml in REPO.glob("modules/**/src/akn/**/*.xml"):
        doc = etree.parse(str(xml))
        assert schema.validate(doc), f"Akoma schema validation failed for {xml}: {schema.error_log.last_error}"
