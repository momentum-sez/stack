"""
PHOENIX Configuration System

Unified configuration management with YAML files, environment variables,
validation, and runtime updates.

Configuration Sources (in order of precedence):
    1. Environment variables (PHOENIX_*)
    2. Runtime overrides
    3. User config file (~/.phoenix/config.yaml)
    4. Project config file (./phoenix.yaml)
    5. Default values

Copyright (c) 2024 Momentum. All rights reserved.
"""

from __future__ import annotations

import os
import threading
from dataclasses import dataclass, field
from decimal import Decimal
from pathlib import Path
from typing import Any, Callable, Dict, Generic, List, Optional, Set, TypeVar, Union

import yaml

T = TypeVar("T")


class ConfigError(Exception):
    """Configuration error."""
    pass


class ValidationError(ConfigError):
    """Configuration validation error."""
    pass


@dataclass
class ConfigValue(Generic[T]):
    """
    A single configuration value with metadata.

    Supports default values, environment variable binding,
    validation, and change callbacks.
    """
    default: T
    env_var: Optional[str] = None
    description: str = ""
    validator: Optional[Callable[[T], bool]] = None
    secret: bool = False  # Don't log if True
    _value: Optional[T] = field(default=None, repr=False)
    _callbacks: List[Callable[[T, T], None]] = field(default_factory=list, repr=False)

    def get(self) -> T:
        """Get the current value."""
        # Check environment variable first
        if self.env_var and self.env_var in os.environ:
            env_value = os.environ[self.env_var]
            return self._coerce(env_value)

        # Return set value or default
        return self._value if self._value is not None else self.default

    def set(self, value: T) -> None:
        """Set the value with validation."""
        if self.validator and not self.validator(value):
            raise ValidationError(f"Invalid value for config: {value}")

        old_value = self._value
        self._value = value

        # Notify callbacks
        for callback in self._callbacks:
            callback(old_value, value)

    def _coerce(self, value: str) -> T:
        """Coerce string value to target type."""
        target_type = type(self.default)

        if target_type == bool:
            return value.lower() in ("true", "1", "yes", "on")  # type: ignore
        elif target_type == int:
            return int(value)  # type: ignore
        elif target_type == float:
            return float(value)  # type: ignore
        elif target_type == Decimal:
            return Decimal(value)  # type: ignore
        elif target_type == list:
            return value.split(",")  # type: ignore
        else:
            return value  # type: ignore

    def on_change(self, callback: Callable[[T, T], None]) -> None:
        """Register a change callback."""
        self._callbacks.append(callback)


@dataclass
class TensorConfig:
    """Configuration for the Compliance Tensor."""
    cache_ttl_seconds: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=300,
        env_var="PHOENIX_TENSOR_CACHE_TTL",
        description="Tensor cache TTL in seconds",
        validator=lambda x: x > 0,
    ))
    max_sparse_cells: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=1000000,
        env_var="PHOENIX_TENSOR_MAX_CELLS",
        description="Maximum number of sparse cells",
        validator=lambda x: x > 0,
    ))
    merkle_batch_size: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=1000,
        env_var="PHOENIX_TENSOR_MERKLE_BATCH",
        description="Batch size for Merkle tree computation",
        validator=lambda x: x > 0,
    ))


@dataclass
class VMConfig:
    """Configuration for the Smart Asset VM."""
    gas_limit_default: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=10000000,
        env_var="PHOENIX_VM_GAS_LIMIT",
        description="Default gas limit for VM execution",
        validator=lambda x: x > 0,
    ))
    stack_depth_max: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=1024,
        env_var="PHOENIX_VM_STACK_DEPTH",
        description="Maximum stack depth",
        validator=lambda x: 0 < x <= 4096,
    ))
    memory_limit_bytes: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=1024 * 1024,  # 1MB
        env_var="PHOENIX_VM_MEMORY_LIMIT",
        description="Maximum memory in bytes",
        validator=lambda x: x > 0,
    ))
    enable_debug_opcodes: ConfigValue[bool] = field(default_factory=lambda: ConfigValue(
        default=False,
        env_var="PHOENIX_VM_DEBUG",
        description="Enable debug opcodes (development only)",
    ))


@dataclass
class WatcherConfig:
    """Configuration for the Watcher Economy."""
    min_collateral_usd: ConfigValue[Decimal] = field(default_factory=lambda: ConfigValue(
        default=Decimal("1000"),
        env_var="PHOENIX_WATCHER_MIN_COLLATERAL",
        description="Minimum collateral in USD",
        validator=lambda x: x > 0,
    ))
    slash_percentage_default: ConfigValue[Decimal] = field(default_factory=lambda: ConfigValue(
        default=Decimal("0.10"),
        env_var="PHOENIX_WATCHER_SLASH_PCT",
        description="Default slash percentage (0-1)",
        validator=lambda x: Decimal("0") <= x <= Decimal("1"),
    ))
    attestation_timeout_seconds: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=300,
        env_var="PHOENIX_WATCHER_ATTEST_TIMEOUT",
        description="Attestation timeout in seconds",
        validator=lambda x: x > 0,
    ))
    quorum_percentage: ConfigValue[Decimal] = field(default_factory=lambda: ConfigValue(
        default=Decimal("0.67"),
        env_var="PHOENIX_WATCHER_QUORUM",
        description="Required quorum percentage",
        validator=lambda x: Decimal("0") < x <= Decimal("1"),
    ))


@dataclass
class AnchorConfig:
    """Configuration for L1 Anchoring."""
    confirmation_blocks_ethereum: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=12,
        env_var="PHOENIX_ANCHOR_ETH_CONFIRMS",
        description="Required confirmations on Ethereum",
        validator=lambda x: x > 0,
    ))
    confirmation_blocks_arbitrum: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=1,
        env_var="PHOENIX_ANCHOR_ARB_CONFIRMS",
        description="Required confirmations on Arbitrum",
        validator=lambda x: x > 0,
    ))
    gas_price_multiplier: ConfigValue[Decimal] = field(default_factory=lambda: ConfigValue(
        default=Decimal("1.2"),
        env_var="PHOENIX_ANCHOR_GAS_MULT",
        description="Gas price multiplier for priority",
        validator=lambda x: x >= Decimal("1"),
    ))
    max_retry_attempts: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=3,
        env_var="PHOENIX_ANCHOR_MAX_RETRIES",
        description="Maximum anchor submission retries",
        validator=lambda x: x >= 0,
    ))


@dataclass
class MigrationConfig:
    """Configuration for Asset Migration."""
    timeout_seconds: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=3600,
        env_var="PHOENIX_MIGRATION_TIMEOUT",
        description="Migration timeout in seconds",
        validator=lambda x: x > 0,
    ))
    max_concurrent_migrations: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=100,
        env_var="PHOENIX_MIGRATION_MAX_CONCURRENT",
        description="Maximum concurrent migrations",
        validator=lambda x: x > 0,
    ))
    compensation_retry_limit: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=5,
        env_var="PHOENIX_MIGRATION_COMP_RETRIES",
        description="Compensation retry limit",
        validator=lambda x: x >= 0,
    ))


@dataclass
class SecurityConfig:
    """Configuration for Security Layer."""
    nonce_ttl_seconds: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=300,
        env_var="PHOENIX_SECURITY_NONCE_TTL",
        description="Nonce TTL in seconds",
        validator=lambda x: x > 0,
    ))
    rate_limit_requests_per_second: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=100,
        env_var="PHOENIX_SECURITY_RATE_LIMIT",
        description="Rate limit (requests/second)",
        validator=lambda x: x > 0,
    ))
    time_lock_min_seconds: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=60,
        env_var="PHOENIX_SECURITY_TIMELOCK_MIN",
        description="Minimum time lock in seconds",
        validator=lambda x: x >= 0,
    ))


@dataclass
class ObservabilityConfig:
    """Configuration for Observability."""
    log_level: ConfigValue[str] = field(default_factory=lambda: ConfigValue(
        default="info",
        env_var="PHOENIX_LOG_LEVEL",
        description="Log level (debug, info, warning, error)",
        validator=lambda x: x in ("debug", "info", "warning", "error", "critical"),
    ))
    log_format: ConfigValue[str] = field(default_factory=lambda: ConfigValue(
        default="json",
        env_var="PHOENIX_LOG_FORMAT",
        description="Log format (json, text)",
        validator=lambda x: x in ("json", "text"),
    ))
    enable_tracing: ConfigValue[bool] = field(default_factory=lambda: ConfigValue(
        default=True,
        env_var="PHOENIX_TRACING_ENABLED",
        description="Enable distributed tracing",
    ))
    metrics_port: ConfigValue[int] = field(default_factory=lambda: ConfigValue(
        default=9090,
        env_var="PHOENIX_METRICS_PORT",
        description="Prometheus metrics port",
        validator=lambda x: 1024 <= x <= 65535,
    ))


@dataclass
class PhoenixConfig:
    """
    Root configuration for PHOENIX.

    Aggregates all component configurations and provides
    loading/saving functionality.
    """
    tensor: TensorConfig = field(default_factory=TensorConfig)
    vm: VMConfig = field(default_factory=VMConfig)
    watcher: WatcherConfig = field(default_factory=WatcherConfig)
    anchor: AnchorConfig = field(default_factory=AnchorConfig)
    migration: MigrationConfig = field(default_factory=MigrationConfig)
    security: SecurityConfig = field(default_factory=SecurityConfig)
    observability: ObservabilityConfig = field(default_factory=ObservabilityConfig)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        def extract_values(obj: Any) -> Any:
            if isinstance(obj, ConfigValue):
                return obj.get()
            elif hasattr(obj, "__dataclass_fields__"):
                return {k: extract_values(getattr(obj, k)) for k in obj.__dataclass_fields__}
            return obj

        return extract_values(self)

    def to_yaml(self) -> str:
        """Convert to YAML string."""
        return yaml.dump(self.to_dict(), default_flow_style=False)


class ConfigManager:
    """
    Configuration manager with file loading and environment binding.

    Thread-safe singleton that manages configuration lifecycle.
    """

    _instance: Optional["ConfigManager"] = None
    _lock = threading.Lock()

    def __new__(cls) -> "ConfigManager":
        with cls._lock:
            if cls._instance is None:
                cls._instance = super().__new__(cls)
                cls._instance._initialized = False
            return cls._instance

    def __init__(self):
        if self._initialized:
            return

        self._config = PhoenixConfig()
        self._config_paths: List[Path] = []
        self._watchers: List[Callable[[PhoenixConfig], None]] = []
        self._initialized = True

    @property
    def config(self) -> PhoenixConfig:
        """Get the current configuration."""
        return self._config

    def load_from_file(self, path: Union[str, Path]) -> None:
        """Load configuration from a YAML file."""
        path = Path(path)
        if not path.exists():
            raise ConfigError(f"Configuration file not found: {path}")

        with open(path) as f:
            data = yaml.safe_load(f)

        if data:
            self._apply_dict(data)
            self._config_paths.append(path)

    def load_from_env(self) -> None:
        """Load configuration from environment variables."""
        # Environment variables are automatically read by ConfigValue.get()
        pass

    def load_defaults(self) -> None:
        """Load default configuration files if they exist."""
        default_paths = [
            Path("phoenix.yaml"),
            Path("config/phoenix.yaml"),
            Path.home() / ".phoenix" / "config.yaml",
        ]

        for path in default_paths:
            if path.exists():
                try:
                    self.load_from_file(path)
                except Exception:
                    pass  # Ignore errors in default config loading

    def _apply_dict(self, data: Dict[str, Any], prefix: str = "") -> None:
        """Apply dictionary values to configuration."""
        def apply_to_config(config_obj: Any, values: Dict[str, Any]) -> None:
            for key, value in values.items():
                if hasattr(config_obj, key):
                    attr = getattr(config_obj, key)
                    if isinstance(attr, ConfigValue):
                        attr.set(value)
                    elif hasattr(attr, "__dataclass_fields__") and isinstance(value, dict):
                        apply_to_config(attr, value)

        apply_to_config(self._config, data)

    def set(self, path: str, value: Any) -> None:
        """
        Set a configuration value by path.

        Example: config.set("vm.gas_limit_default", 5000000)
        """
        parts = path.split(".")
        obj = self._config

        for part in parts[:-1]:
            obj = getattr(obj, part)

        attr = getattr(obj, parts[-1])
        if isinstance(attr, ConfigValue):
            attr.set(value)
        else:
            raise ConfigError(f"Invalid config path: {path}")

    def get(self, path: str) -> Any:
        """
        Get a configuration value by path.

        Example: config.get("vm.gas_limit_default")
        """
        parts = path.split(".")
        obj = self._config

        for part in parts:
            obj = getattr(obj, part)

        if isinstance(obj, ConfigValue):
            return obj.get()
        return obj

    def watch(self, callback: Callable[[PhoenixConfig], None]) -> None:
        """Register a callback for configuration changes."""
        self._watchers.append(callback)

    def reload(self) -> None:
        """Reload configuration from all loaded files."""
        for path in self._config_paths:
            if path.exists():
                self.load_from_file(path)

        for watcher in self._watchers:
            watcher(self._config)

    def validate(self) -> List[str]:
        """
        Validate all configuration values.

        Returns list of validation errors.
        """
        errors: List[str] = []

        def validate_config(obj: Any, path: str = "") -> None:
            if isinstance(obj, ConfigValue):
                try:
                    value = obj.get()
                    if obj.validator and not obj.validator(value):
                        errors.append(f"{path}: validation failed for value {value}")
                except Exception as e:
                    errors.append(f"{path}: {e}")
            elif hasattr(obj, "__dataclass_fields__"):
                for field_name in obj.__dataclass_fields__:
                    field_path = f"{path}.{field_name}" if path else field_name
                    validate_config(getattr(obj, field_name), field_path)

        validate_config(self._config)
        return errors

    def export_schema(self) -> Dict[str, Any]:
        """Export configuration schema for documentation."""
        schema: Dict[str, Any] = {"properties": {}}

        def extract_schema(obj: Any, properties: Dict[str, Any]) -> None:
            if isinstance(obj, ConfigValue):
                properties["type"] = type(obj.default).__name__
                properties["default"] = str(obj.default)
                properties["description"] = obj.description
                if obj.env_var:
                    properties["env_var"] = obj.env_var
            elif hasattr(obj, "__dataclass_fields__"):
                for field_name in obj.__dataclass_fields__:
                    properties[field_name] = {}
                    extract_schema(getattr(obj, field_name), properties[field_name])

        extract_schema(self._config, schema["properties"])
        return schema


def get_config() -> PhoenixConfig:
    """Get the current PHOENIX configuration."""
    return ConfigManager().config


def get_config_manager() -> ConfigManager:
    """Get the configuration manager instance."""
    return ConfigManager()
