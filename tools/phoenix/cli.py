#!/usr/bin/env python3
"""
PHOENIX Unified CLI

Command-line interface for the PHOENIX Smart Asset Operating System.
Provides unified access to all PHOENIX subsystems with consistent UX.

Usage:
    phoenix <command> [subcommand] [options]

Commands:
    tensor      Compliance tensor operations
    vm          Smart Asset VM execution
    manifold    Compliance path planning
    migration   Asset migration management
    bridge      Multi-hop corridor bridging
    anchor      L1 checkpoint anchoring
    watcher     Watcher economy management
    config      Configuration management
    health      Health checks and diagnostics

Copyright (c) 2024 Momentum. All rights reserved.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict
from datetime import datetime, timezone
from decimal import Decimal
from enum import Enum
from typing import Any, Callable, Dict, List, Optional

# Version
__version__ = "0.4.44"


class OutputFormat(Enum):
    """Output format options."""
    JSON = "json"
    YAML = "yaml"
    TABLE = "table"
    TEXT = "text"


class CLIError(Exception):
    """CLI error with exit code."""
    def __init__(self, message: str, exit_code: int = 1):
        super().__init__(message)
        self.exit_code = exit_code


def format_output(data: Any, fmt: OutputFormat = OutputFormat.JSON) -> str:
    """Format data for output."""
    if fmt == OutputFormat.JSON:
        return json.dumps(data, indent=2, default=str)
    elif fmt == OutputFormat.YAML:
        import yaml
        return yaml.dump(data, default_flow_style=False)
    elif fmt == OutputFormat.TABLE:
        return _format_table(data)
    else:
        return str(data)


def _format_table(data: Any) -> str:
    """Format data as ASCII table."""
    if isinstance(data, list) and data and isinstance(data[0], dict):
        headers = list(data[0].keys())
        rows = [[str(row.get(h, ""))[:40] for h in headers] for row in data]
        widths = [max(len(h), max(len(r[i]) for r in rows)) for i, h in enumerate(headers)]

        lines = []
        header_line = " | ".join(h.ljust(widths[i]) for i, h in enumerate(headers))
        lines.append(header_line)
        lines.append("-+-".join("-" * w for w in widths))
        for row in rows:
            lines.append(" | ".join(c.ljust(widths[i]) for i, c in enumerate(row)))
        return "\n".join(lines)
    elif isinstance(data, dict):
        return "\n".join(f"{k}: {v}" for k, v in data.items())
    return str(data)


class PhoenixCLI:
    """Main CLI application."""

    def __init__(self):
        self.parser = argparse.ArgumentParser(
            prog="phoenix",
            description="PHOENIX Smart Asset Operating System CLI",
            formatter_class=argparse.RawDescriptionHelpFormatter,
        )
        self.parser.add_argument(
            "--version", "-V",
            action="version",
            version=f"phoenix {__version__}",
        )
        self.parser.add_argument(
            "--format", "-f",
            choices=["json", "yaml", "table", "text"],
            default="json",
            help="Output format (default: json)",
        )
        self.parser.add_argument(
            "--quiet", "-q",
            action="store_true",
            help="Suppress non-essential output",
        )

        self.subparsers = self.parser.add_subparsers(dest="command", help="Commands")
        self._register_commands()

    def _register_commands(self) -> None:
        """Register all command groups."""
        self._register_tensor_commands()
        self._register_vm_commands()
        self._register_manifold_commands()
        self._register_migration_commands()
        self._register_watcher_commands()
        self._register_anchor_commands()
        self._register_config_commands()
        self._register_health_commands()

    def _register_tensor_commands(self) -> None:
        """Register tensor subcommands."""
        tensor = self.subparsers.add_parser("tensor", help="Compliance tensor operations")
        tensor_sub = tensor.add_subparsers(dest="subcommand")

        # tensor query
        query = tensor_sub.add_parser("query", help="Query compliance state")
        query.add_argument("--asset", "-a", required=True, help="Asset ID")
        query.add_argument("--jurisdiction", "-j", required=True, help="Jurisdiction ID")
        query.add_argument("--domain", "-d", help="Compliance domain")
        query.add_argument("--time", "-t", type=int, help="Time quantum (Unix timestamp)")

        # tensor set
        set_cmd = tensor_sub.add_parser("set", help="Set compliance state")
        set_cmd.add_argument("--asset", "-a", required=True, help="Asset ID")
        set_cmd.add_argument("--jurisdiction", "-j", required=True, help="Jurisdiction ID")
        set_cmd.add_argument("--domain", "-d", required=True, help="Compliance domain")
        set_cmd.add_argument("--state", "-s", required=True, help="Compliance state")
        set_cmd.add_argument("--reason", "-r", help="Reason code")

        # tensor merkle
        merkle = tensor_sub.add_parser("merkle", help="Compute Merkle root")
        merkle.add_argument("--asset", "-a", help="Filter by asset ID")
        merkle.add_argument("--jurisdiction", "-j", help="Filter by jurisdiction")

    def _register_vm_commands(self) -> None:
        """Register VM subcommands."""
        vm = self.subparsers.add_parser("vm", help="Smart Asset VM operations")
        vm_sub = vm.add_subparsers(dest="subcommand")

        # vm execute
        execute = vm_sub.add_parser("execute", help="Execute bytecode")
        execute.add_argument("--bytecode", "-b", required=True, help="Hex bytecode or file path")
        execute.add_argument("--gas-limit", "-g", type=int, default=1000000, help="Gas limit")
        execute.add_argument("--caller", "-c", help="Caller DID")
        execute.add_argument("--jurisdiction", "-j", help="Jurisdiction ID")

        # vm assemble
        assemble = vm_sub.add_parser("assemble", help="Assemble from mnemonics")
        assemble.add_argument("--input", "-i", required=True, help="Assembly file")
        assemble.add_argument("--output", "-o", help="Output file")

        # vm disassemble
        disassemble = vm_sub.add_parser("disassemble", help="Disassemble bytecode")
        disassemble.add_argument("--bytecode", "-b", required=True, help="Hex bytecode")

    def _register_manifold_commands(self) -> None:
        """Register manifold subcommands."""
        manifold = self.subparsers.add_parser("manifold", help="Compliance path planning")
        manifold_sub = manifold.add_subparsers(dest="subcommand")

        # manifold path
        path = manifold_sub.add_parser("path", help="Find compliant path")
        path.add_argument("--source", "-s", required=True, help="Source jurisdiction")
        path.add_argument("--target", "-t", required=True, help="Target jurisdiction")
        path.add_argument("--asset", "-a", help="Asset ID for compliance check")
        path.add_argument("--max-hops", type=int, default=5, help="Maximum hops")
        path.add_argument("--max-cost", type=float, help="Maximum cost in USD")

        # manifold corridors
        corridors = manifold_sub.add_parser("corridors", help="List corridors")
        corridors.add_argument("--jurisdiction", "-j", help="Filter by jurisdiction")
        corridors.add_argument("--active-only", action="store_true", help="Show only active")

    def _register_migration_commands(self) -> None:
        """Register migration subcommands."""
        migration = self.subparsers.add_parser("migration", help="Asset migration management")
        migration_sub = migration.add_subparsers(dest="subcommand")

        # migration start
        start = migration_sub.add_parser("start", help="Start a migration")
        start.add_argument("--asset", "-a", required=True, help="Asset ID")
        start.add_argument("--source", "-s", required=True, help="Source jurisdiction")
        start.add_argument("--target", "-t", required=True, help="Target jurisdiction")
        start.add_argument("--owner", "-o", required=True, help="Owner DID")

        # migration status
        status = migration_sub.add_parser("status", help="Check migration status")
        status.add_argument("--id", "-i", required=True, help="Migration ID")

        # migration cancel
        cancel = migration_sub.add_parser("cancel", help="Cancel a migration")
        cancel.add_argument("--id", "-i", required=True, help="Migration ID")
        cancel.add_argument("--reason", "-r", required=True, help="Cancellation reason")

        # migration list
        list_cmd = migration_sub.add_parser("list", help="List migrations")
        list_cmd.add_argument("--state", help="Filter by state")
        list_cmd.add_argument("--asset", "-a", help="Filter by asset")

    def _register_watcher_commands(self) -> None:
        """Register watcher subcommands."""
        watcher = self.subparsers.add_parser("watcher", help="Watcher economy management")
        watcher_sub = watcher.add_subparsers(dest="subcommand")

        # watcher register
        register = watcher_sub.add_parser("register", help="Register as watcher")
        register.add_argument("--did", "-d", required=True, help="Watcher DID")
        register.add_argument("--collateral", "-c", type=float, required=True, help="Collateral in USD")
        register.add_argument("--jurisdictions", "-j", nargs="+", help="Covered jurisdictions")

        # watcher attest
        attest = watcher_sub.add_parser("attest", help="Create attestation")
        attest.add_argument("--watcher-did", "-w", required=True, help="Watcher DID")
        attest.add_argument("--target", "-t", required=True, help="Target digest")
        attest.add_argument("--scope", "-s", required=True, help="Attestation scope")

        # watcher list
        list_cmd = watcher_sub.add_parser("list", help="List watchers")
        list_cmd.add_argument("--jurisdiction", "-j", help="Filter by jurisdiction")
        list_cmd.add_argument("--min-collateral", type=float, help="Minimum collateral")

        # watcher reputation
        reputation = watcher_sub.add_parser("reputation", help="Get watcher reputation")
        reputation.add_argument("--did", "-d", required=True, help="Watcher DID")

    def _register_anchor_commands(self) -> None:
        """Register anchor subcommands."""
        anchor = self.subparsers.add_parser("anchor", help="L1 checkpoint anchoring")
        anchor_sub = anchor.add_subparsers(dest="subcommand")

        # anchor submit
        submit = anchor_sub.add_parser("submit", help="Submit checkpoint to L1")
        submit.add_argument("--checkpoint", "-c", required=True, help="Checkpoint digest")
        submit.add_argument("--chain", default="ethereum", help="Target chain")
        submit.add_argument("--contract", help="Contract address")

        # anchor status
        status = anchor_sub.add_parser("status", help="Check anchor status")
        status.add_argument("--id", "-i", required=True, help="Anchor ID")

        # anchor verify
        verify = anchor_sub.add_parser("verify", help="Verify checkpoint inclusion")
        verify.add_argument("--checkpoint", "-c", required=True, help="Checkpoint digest")
        verify.add_argument("--chain", default="ethereum", help="Target chain")

    def _register_config_commands(self) -> None:
        """Register config subcommands."""
        config = self.subparsers.add_parser("config", help="Configuration management")
        config_sub = config.add_subparsers(dest="subcommand")

        # config get
        get = config_sub.add_parser("get", help="Get configuration value")
        get.add_argument("path", help="Config path (e.g., vm.gas_limit_default)")

        # config set
        set_cmd = config_sub.add_parser("set", help="Set configuration value")
        set_cmd.add_argument("path", help="Config path")
        set_cmd.add_argument("value", help="Value to set")

        # config show
        config_sub.add_parser("show", help="Show all configuration")

        # config validate
        config_sub.add_parser("validate", help="Validate configuration")

        # config schema
        config_sub.add_parser("schema", help="Export configuration schema")

    def _register_health_commands(self) -> None:
        """Register health subcommands."""
        health = self.subparsers.add_parser("health", help="Health checks and diagnostics")
        health_sub = health.add_subparsers(dest="subcommand")

        # health check
        health_sub.add_parser("check", help="Run full health check")

        # health live
        health_sub.add_parser("live", help="Liveness probe")

        # health ready
        health_sub.add_parser("ready", help="Readiness probe")

        # health version
        health_sub.add_parser("version", help="Show version info")

    def run(self, args: Optional[List[str]] = None) -> int:
        """Run the CLI."""
        parsed = self.parser.parse_args(args)

        if not parsed.command:
            self.parser.print_help()
            return 0

        try:
            fmt = OutputFormat(parsed.format)
            result = self._dispatch(parsed)

            if result is not None:
                print(format_output(result, fmt))

            return 0

        except CLIError as e:
            if not parsed.quiet:
                print(f"Error: {e}", file=sys.stderr)
            return e.exit_code

        except Exception as e:
            if not parsed.quiet:
                print(f"Error: {e}", file=sys.stderr)
            return 1

    def _dispatch(self, args: argparse.Namespace) -> Any:
        """Dispatch command to handler."""
        cmd = args.command
        subcmd = getattr(args, "subcommand", None)

        handler_name = f"_handle_{cmd}_{subcmd}" if subcmd else f"_handle_{cmd}"
        handler = getattr(self, handler_name, None)

        if handler is None:
            raise CLIError(f"Unknown command: {cmd} {subcmd or ''}")

        return handler(args)

    # Config handlers
    def _handle_config_get(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.config import get_config_manager
        mgr = get_config_manager()
        return {"path": args.path, "value": mgr.get(args.path)}

    def _handle_config_set(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.config import get_config_manager
        mgr = get_config_manager()
        mgr.set(args.path, args.value)
        return {"path": args.path, "value": args.value, "status": "updated"}

    def _handle_config_show(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.config import get_config_manager
        mgr = get_config_manager()
        return mgr.config.to_dict()

    def _handle_config_validate(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.config import get_config_manager
        mgr = get_config_manager()
        errors = mgr.validate()
        return {"valid": len(errors) == 0, "errors": errors}

    def _handle_config_schema(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.config import get_config_manager
        mgr = get_config_manager()
        return mgr.export_schema()

    # Health handlers
    def _handle_health_check(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.health import get_health_checker
        checker = get_health_checker()
        report = checker.deep_health()
        return report.to_dict()

    def _handle_health_live(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.health import get_health_checker
        checker = get_health_checker()
        result = checker.liveness()
        return result.to_dict()

    def _handle_health_ready(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.health import get_health_checker
        checker = get_health_checker()
        result = checker.readiness()
        return result.to_dict()

    def _handle_health_version(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.health import get_health_checker
        checker = get_health_checker()
        return checker.get_version_info()

    # Tensor handlers
    def _handle_tensor_query(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.tensor import ComplianceDomain, ComplianceTensorV2, TensorCoordinate
        tensor = ComplianceTensorV2()

        domain = ComplianceDomain[args.domain.upper()] if args.domain else ComplianceDomain.KYC_AML
        time_quantum = args.time or int(datetime.now(timezone.utc).timestamp())

        coord = TensorCoordinate(
            asset_id=args.asset,
            jurisdiction_id=args.jurisdiction,
            domain=domain,
            time_quantum=time_quantum,
        )

        cell = tensor.get(coord)
        if cell:
            return {
                "asset_id": args.asset,
                "jurisdiction_id": args.jurisdiction,
                "domain": domain.value,
                "state": cell.state.value,
                "reason_code": cell.reason_code,
                "attestations": len(cell.attestations),
            }
        return {"status": "not_found", "coordinate": str(coord)}

    def _handle_tensor_merkle(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.tensor import ComplianceTensorV2
        tensor = ComplianceTensorV2()
        root = tensor.merkle_root()
        return {"merkle_root": root, "cell_count": len(tensor._cells)}

    # VM handlers
    def _handle_vm_execute(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.vm import ExecutionContext, SmartAssetVM

        vm = SmartAssetVM()

        # Load bytecode
        if args.bytecode.startswith("0x"):
            bytecode = bytes.fromhex(args.bytecode[2:])
        elif args.bytecode.endswith(".hex"):
            with open(args.bytecode) as f:
                bytecode = bytes.fromhex(f.read().strip())
        else:
            bytecode = bytes.fromhex(args.bytecode)

        ctx = ExecutionContext(
            caller=args.caller or "did:cli:anonymous",
            origin=args.caller or "did:cli:anonymous",
            jurisdiction_id=args.jurisdiction or "global",
            timestamp=int(datetime.now(timezone.utc).timestamp()),
            block_height=0,
            gas_limit=args.gas_limit,
            gas_price=1,
        )

        result = vm.execute(bytecode, ctx)

        return {
            "success": result.success,
            "gas_used": result.gas_used,
            "return_data": result.return_data.hex() if result.return_data else None,
            "error": result.error,
            "logs": result.logs,
        }

    def _handle_vm_disassemble(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.vm import Assembler

        if args.bytecode.startswith("0x"):
            bytecode = bytes.fromhex(args.bytecode[2:])
        else:
            bytecode = bytes.fromhex(args.bytecode)

        instructions = Assembler.disassemble(bytecode)
        return {"instructions": [str(i) for i in instructions]}

    # Watcher handlers
    def _handle_watcher_list(self, args: argparse.Namespace) -> Any:
        from tools.phoenix.watcher import WatcherRegistry
        registry = WatcherRegistry()

        watchers = []
        for watcher in registry._watchers.values():
            if args.jurisdiction and args.jurisdiction not in watcher.jurisdiction_ids:
                continue
            if args.min_collateral and watcher.total_collateral_usd < Decimal(str(args.min_collateral)):
                continue
            watchers.append({
                "did": watcher.did,
                "collateral_usd": str(watcher.total_collateral_usd),
                "jurisdictions": list(watcher.jurisdiction_ids),
                "reputation": watcher.reputation_score,
            })

        return {"watchers": watchers, "count": len(watchers)}

    # Migration handlers
    def _handle_migration_list(self, args: argparse.Namespace) -> Any:
        return {"migrations": [], "count": 0, "note": "No active migrations"}


def main() -> int:
    """CLI entry point."""
    cli = PhoenixCLI()
    return cli.run()


if __name__ == "__main__":
    sys.exit(main())
