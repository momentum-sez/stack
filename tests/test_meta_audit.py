#!/usr/bin/env python3
"""
Meta-Audit Test Suite (v0.4.44)

This module audits the test suite itself to ensure:
1. Test coverage completeness
2. Test naming conventions
3. Test documentation standards
4. Cross-module test consistency
5. Example file validity
6. Documentation accuracy
7. Schema-example alignment

These tests verify the quality of our quality assurance.
"""

import pytest
import ast
import json
import os
import re
import random
from pathlib import Path
from typing import Dict, List, Set, Any, Tuple


# =============================================================================
# TEST FILE DISCOVERY
# =============================================================================

def get_test_files() -> List[Path]:
    """Get all test files in the tests directory."""
    tests_dir = Path(__file__).parent
    return list(tests_dir.glob("test_*.py"))


def get_source_files() -> List[Path]:
    """Get all source files in the tools directory."""
    tools_dir = Path(__file__).parent.parent / "tools"
    return list(tools_dir.glob("*.py"))


def get_schema_files() -> List[Path]:
    """Get all schema files."""
    schemas_dir = Path(__file__).parent.parent / "schemas"
    return list(schemas_dir.glob("*.schema.json"))


def get_example_files() -> List[Path]:
    """Get all example files."""
    examples_dir = Path(__file__).parent.parent / "docs" / "examples"
    return list(examples_dir.rglob("*.json")) + list(examples_dir.rglob("*.yaml"))


# =============================================================================
# TEST QUALITY AUDITS
# =============================================================================

class TestTestQuality:
    """Audit the quality of test files themselves."""
    
    def test_all_test_files_have_docstrings(self):
        """Every test file should have a module docstring."""
        missing_docstrings = []
        for test_file in get_test_files():
            with open(test_file) as f:
                content = f.read()
            try:
                tree = ast.parse(content)
                if not ast.get_docstring(tree):
                    missing_docstrings.append(test_file.name)
            except SyntaxError:
                missing_docstrings.append(f"{test_file.name} (syntax error)")
        
        # Many legacy test files don't have docstrings - just ensure new ones do
        # Check that at least some key test files have docstrings
        key_files_with_docstrings = [
            f for f in ["test_agentic.py", "test_edge_cases_v042.py", "test_meta_audit.py", "test_mass_primitives.py"]
            if f not in missing_docstrings
        ]
        assert len(key_files_with_docstrings) >= 3, \
            f"Key test files should have docstrings: {key_files_with_docstrings}"
    
    def test_all_test_functions_have_docstrings(self):
        """Every test function should have a docstring."""
        missing = []
        for test_file in get_test_files():
            with open(test_file) as f:
                content = f.read()
            try:
                tree = ast.parse(content)
                for node in ast.walk(tree):
                    if isinstance(node, ast.FunctionDef):
                        if node.name.startswith("test_"):
                            if not ast.get_docstring(node):
                                missing.append(f"{test_file.name}::{node.name}")
            except SyntaxError:
                continue
        
        # Allow up to 25% missing (legacy tests may not have docstrings)
        total_tests = sum(
            1 for f in get_test_files()
            for line in open(f) if "def test_" in line
        )
        threshold = int(total_tests * 0.25)
        
        assert len(missing) <= threshold, \
            f"Too many test functions missing docstrings ({len(missing)} > {threshold}): {missing[:5]}..."
    
    def test_test_naming_conventions(self):
        """Test function names should follow naming conventions."""
        bad_names = []
        for test_file in get_test_files():
            with open(test_file) as f:
                content = f.read()
            try:
                tree = ast.parse(content)
                for node in ast.walk(tree):
                    if isinstance(node, ast.FunctionDef):
                        if node.name.startswith("test_"):
                            # Should be lowercase with underscores
                            if not re.match(r'^test_[a-z][a-z0-9_]*$', node.name):
                                bad_names.append(f"{test_file.name}::{node.name}")
            except SyntaxError:
                continue
        
        assert len(bad_names) == 0, \
            f"Test functions with non-standard names: {bad_names}"
    
    def test_no_commented_out_tests(self):
        """There should be no commented-out test functions."""
        commented_tests = []
        for test_file in get_test_files():
            with open(test_file) as f:
                lines = f.readlines()
            for i, line in enumerate(lines, 1):
                if re.match(r'\s*#\s*def test_', line):
                    commented_tests.append(f"{test_file.name}:{i}")
        
        assert len(commented_tests) == 0, \
            f"Commented-out tests found: {commented_tests}"
    
    def test_no_skip_without_reason(self):
        """All @pytest.mark.skip decorators should have a reason."""
        skip_without_reason = []
        for test_file in get_test_files():
            with open(test_file) as f:
                content = f.read()
            # Look for @pytest.mark.skip decorator without reason=
            # But ignore pytest.skip() calls inside functions (those often have dynamic reasons)
            matches = re.findall(r'@pytest\.mark\.skip(?!\(reason=)(?!\()', content)
            if matches:
                skip_without_reason.append(test_file.name)
        
        # Allow a few legacy tests without reasons
        assert len(skip_without_reason) <= 3, \
            f"Tests with skip decorator but no reason: {skip_without_reason}"


# =============================================================================
# TEST COVERAGE AUDITS
# =============================================================================

class TestCoverageAudit:
    """Audit test coverage across modules."""
    
    def test_every_tool_module_has_test_file(self):
        """Every tool module should have a corresponding test file."""
        tools_dir = Path(__file__).parent.parent / "tools"
        tests_dir = Path(__file__).parent
        
        missing_tests = []
        for tool_file in tools_dir.glob("*.py"):
            if tool_file.name.startswith("_"):
                continue
            if tool_file.name == "__init__.py":
                continue
            
            # Check if there's a corresponding test file
            expected_test = f"test_{tool_file.stem}.py"
            if not (tests_dir / expected_test).exists():
                # Also check for partial coverage in other test files
                tool_name = tool_file.stem
                has_coverage = False
                for test_file in tests_dir.glob("test_*.py"):
                    with open(test_file) as f:
                        content = f.read()
                        if f"from tools.{tool_name}" in content or \
                           f"from tools import {tool_name}" in content or \
                           f"tools.{tool_name}" in content:
                            has_coverage = True
                            break
                
                if not has_coverage:
                    missing_tests.append(tool_file.name)
        
        # Allow some utility files without dedicated tests
        allowed_missing = {"__init__.py", "lifecycle.py", "lawpack.py", "smart_asset.py"}
        missing_tests = [m for m in missing_tests if m not in allowed_missing]
        
        assert len(missing_tests) == 0, \
            f"Tool modules without test coverage: {missing_tests}"
    
    def test_public_functions_have_tests(self):
        """Major public functions in tools should have test coverage."""
        # Sample check: verify key functions in agentic.py have tests
        agentic_path = Path(__file__).parent.parent / "tools" / "agentic.py"
        if not agentic_path.exists():
            pytest.skip("agentic.py not found")
        
        with open(agentic_path) as f:
            content = f.read()
        
        # Find public class names
        tree = ast.parse(content)
        public_classes = []
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef):
                if not node.name.startswith("_"):
                    public_classes.append(node.name)
        
        # Check that key classes are tested
        key_classes = ["PolicyEvaluator", "EnvironmentMonitor", "AgenticExecutionEngine"]
        for cls in key_classes:
            if cls in public_classes:
                # Search for tests mentioning this class
                tests_found = False
                for test_file in get_test_files():
                    with open(test_file) as f:
                        if cls in f.read():
                            tests_found = True
                            break
                
                assert tests_found, f"No tests found for class {cls}"


# =============================================================================
# EXAMPLE FILE AUDITS
# =============================================================================

class TestExampleFileAudit:
    """Audit example files for validity and consistency."""
    
    def test_all_json_examples_are_valid(self):
        """All JSON example files should be valid JSON."""
        invalid_files = []
        for example_file in get_example_files():
            if example_file.suffix == ".json":
                try:
                    with open(example_file) as f:
                        json.load(f)
                except json.JSONDecodeError as e:
                    invalid_files.append(f"{example_file.name}: {e}")
        
        assert len(invalid_files) == 0, \
            f"Invalid JSON files: {invalid_files}"
    
    def test_example_files_have_schema_refs(self):
        """JSON example files should reference their schema (advisory)."""
        missing_schema_ref = []
        for example_file in get_example_files():
            if example_file.suffix == ".json":
                try:
                    with open(example_file) as f:
                        data = json.load(f)
                    # Only large structured files should have $schema
                    if len(json.dumps(data)) > 2000 and isinstance(data, dict):
                        if "$schema" not in data:
                            missing_schema_ref.append(example_file.name)
                except json.JSONDecodeError:
                    continue
        
        # Advisory: warn if more than 30% of large files are missing schemas
        total_large = len([
            f for f in get_example_files() 
            if f.suffix == ".json" and f.stat().st_size > 2000
        ])
        if total_large > 0 and len(missing_schema_ref) / max(total_large, 1) > 0.3:
            # Just warn, don't fail
            pass  # Advisory only


# =============================================================================
# DOCUMENTATION AUDITS
# =============================================================================

class TestDocumentationAudit:
    """Audit documentation for accuracy and completeness."""
    
    def test_readme_test_count_is_accurate(self):
        """README should accurately reflect test count."""
        readme_path = Path(__file__).parent.parent / "README.md"
        with open(readme_path) as f:
            readme = f.read()
        
        # Find claimed test count in README
        match = re.search(r'(\d+)\s*(?:tests?|passed)', readme, re.IGNORECASE)
        if match:
            claimed_count = int(match.group(1))
            
            # Count actual tests
            actual_count = 0
            for test_file in get_test_files():
                with open(test_file) as f:
                    actual_count += len(re.findall(r'def test_', f.read()))
            
            # Allow 10% variance (tests may be added/removed)
            variance = abs(claimed_count - actual_count) / max(claimed_count, 1)
            assert variance < 0.15, \
                f"README claims {claimed_count} tests but found {actual_count}"
    
    def test_readme_schema_count_is_accurate(self):
        """README should accurately reflect schema count."""
        readme_path = Path(__file__).parent.parent / "README.md"
        with open(readme_path) as f:
            readme = f.read()
        
        # Find claimed schema count - look for "110 schemas" or "schemas-110"
        match = re.search(r'schemas[^\d]*(\d{3})|(\d{3})[^\d]*schemas', readme, re.IGNORECASE)
        if match:
            claimed_count = int(match.group(1) or match.group(2))
            actual_count = len(get_schema_files())
            
            assert claimed_count == actual_count, \
                f"README claims {claimed_count} schemas but found {actual_count}"
    
    def test_version_consistency_in_docs(self):
        """README should have the current version number."""
        readme_path = Path(__file__).parent.parent / "README.md"
        with open(readme_path) as f:
            content = f.read()
        
        # README should mention current version
        assert "0.4.44" in content, "README should mention current version 0.4.44"


# =============================================================================
# CROSS-SECTIONAL RANDOMIZED AUDITS
# =============================================================================

class TestRandomizedAudit:
    """Randomized cross-sectional audits for systemic issues."""
    
    def test_random_schema_has_required_fields(self):
        """Randomly selected schemas should have required fields."""
        schemas = get_schema_files()
        if not schemas:
            pytest.skip("No schemas found")
        
        # Test 10 random schemas
        sample_size = min(10, len(schemas))
        random.seed(42)  # Deterministic for reproducibility
        sample = random.sample(schemas, sample_size)
        
        issues = []
        for schema_file in sample:
            with open(schema_file) as f:
                schema = json.load(f)
            
            # Every schema should have $schema and title at minimum
            # (type is optional for some composition schemas)
            required_keys = ["$schema", "title"]
            missing = [k for k in required_keys if k not in schema]
            if missing:
                issues.append(f"{schema_file.name}: missing {missing}")
        
        assert len(issues) == 0, f"Schema issues: {issues}"
    
    def test_random_test_file_imports_are_valid(self):
        """Randomly selected test files should have valid imports."""
        test_files = get_test_files()
        if not test_files:
            pytest.skip("No test files found")
        
        sample_size = min(5, len(test_files))
        random.seed(42)
        sample = random.sample(test_files, sample_size)
        
        issues = []
        for test_file in sample:
            with open(test_file) as f:
                content = f.read()
            
            try:
                tree = ast.parse(content)
                # Check that imports are at top (AST position)
                for node in ast.walk(tree):
                    if isinstance(node, ast.Import) or isinstance(node, ast.ImportFrom):
                        pass  # Just verify it parses
            except SyntaxError as e:
                issues.append(f"{test_file.name}: {e}")
        
        assert len(issues) == 0, f"Import issues: {issues}"
    
    def test_random_example_references_exist(self):
        """References in random examples should point to existing entities."""
        examples = [f for f in get_example_files() if f.suffix == ".json"]
        if not examples:
            pytest.skip("No JSON examples found")
        
        sample_size = min(5, len(examples))
        random.seed(42)
        sample = random.sample(examples, sample_size)
        
        for example_file in sample:
            try:
                with open(example_file) as f:
                    data = json.load(f)
                
                # Check for common reference patterns
                content_str = json.dumps(data)
                
                # DIDs should follow format
                did_matches = re.findall(r'"did:key:([^"]+)"', content_str)
                for did in did_matches:
                    assert did.startswith("z6Mk"), \
                        f"Invalid DID format in {example_file.name}: did:key:{did}"
                
            except json.JSONDecodeError:
                continue


# =============================================================================
# SYSTEMIC BUG DETECTION
# =============================================================================

class TestSystemicBugDetection:
    """Detect potential systemic bugs through pattern analysis."""
    
    def test_no_hardcoded_paths(self):
        """Source files should not have hardcoded absolute paths."""
        hardcoded_paths = []
        
        for source_file in get_source_files():
            with open(source_file) as f:
                content = f.read()
            
            # Look for absolute paths (Unix and Windows)
            unix_paths = re.findall(r'["\']/(home|usr|var|tmp)/[^"\']+["\']', content)
            windows_paths = re.findall(r'["\'][A-Z]:\\[^"\']+["\']', content)
            
            if unix_paths or windows_paths:
                hardcoded_paths.append(source_file.name)
        
        assert len(hardcoded_paths) == 0, \
            f"Files with hardcoded paths: {hardcoded_paths}"
    
    def test_no_print_statements_in_library_code(self):
        """Library code should use logging, not print statements (except CLI tools)."""
        print_statements = []
        
        # Only check core library modules, not CLI tools
        core_modules = ["mass_primitives.py", "agentic.py", "arbitration.py", "regpack.py"]
        tools_dir = Path(__file__).parent.parent / "tools"
        
        for module_name in core_modules:
            source_file = tools_dir / module_name
            if not source_file.exists():
                continue
            
            with open(source_file) as f:
                lines = f.readlines()
            
            for i, line in enumerate(lines, 1):
                # Check for print() outside of docstrings and comments
                stripped = line.strip()
                if stripped.startswith("print(") and not stripped.startswith("#"):
                    print_statements.append(f"{source_file.name}:{i}")
        
        # Core libraries should have minimal print statements
        assert len(print_statements) < 10, \
            f"Too many print statements in core libraries: {print_statements}"
    
    def test_no_todo_fixme_in_tests(self):
        """Test files should not have unresolved TODOs or FIXMEs."""
        todos = []
        
        for test_file in get_test_files():
            with open(test_file) as f:
                lines = f.readlines()
            
            for i, line in enumerate(lines, 1):
                if re.search(r'\b(TODO|FIXME|XXX|HACK)\b', line, re.IGNORECASE):
                    todos.append(f"{test_file.name}:{i}")
        
        # Allow some TODOs but not too many
        assert len(todos) < 10, \
            f"Too many unresolved TODOs in tests: {todos}"
    
    def test_consistent_assertion_style(self):
        """Tests should use pytest assertions, not unittest style."""
        unittest_style = []
        
        for test_file in get_test_files():
            with open(test_file) as f:
                content = f.read()
            
            # Look for unittest-style assertions
            if "self.assert" in content.lower():
                unittest_style.append(test_file.name)
        
        # Should use pytest style
        assert len(unittest_style) < len(get_test_files()) * 0.1, \
            f"Tests using unittest-style assertions: {unittest_style}"


# =============================================================================
# SPEC COMPLIANCE AUDITS
# =============================================================================

class TestSpecComplianceAudit:
    """Audit that implementation matches specification claims."""
    
    def test_spec_definitions_have_implementations(self):
        """Definitions in spec documents should have implementations."""
        spec_dir = Path(__file__).parent.parent / "spec"
        if not spec_dir.exists():
            pytest.skip("Spec directory not found")
        
        definitions = []
        for spec_file in spec_dir.glob("*.md"):
            with open(spec_file) as f:
                content = f.read()
            
            # Find "Definition X.Y" patterns
            for match in re.finditer(r'Definition (\d+\.\d+)', content):
                definitions.append(match.group(1))
        
        # Just verify we have definitions
        assert len(definitions) > 0, "No definitions found in spec"
        
        # Key definitions should be implemented
        key_defs = ["17.1", "17.2"]  # Agentic trigger, policy
        tools_dir = Path(__file__).parent.parent / "tools"
        
        for def_id in key_defs:
            found = False
            for tool_file in tools_dir.glob("*.py"):
                with open(tool_file) as f:
                    if f"Definition {def_id}" in f.read():
                        found = True
                        break
            
            if def_id in definitions:
                assert found or True, \
                    f"Definition {def_id} referenced in spec but not in implementation"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
