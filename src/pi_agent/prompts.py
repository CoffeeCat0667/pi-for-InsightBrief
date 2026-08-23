"""Prompt loading from external markdown files."""

from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class PromptSet:
    """可外部加载的提示词集合。"""
    system_main: str = ""
    compaction_system: str = ""
    compaction_initial: str = ""
    compaction_update: str = ""
    compaction_turn_prefix: str = ""
    tool_guidelines: dict[str, str] = field(default_factory=dict)
    extra: dict[str, str] = field(default_factory=dict)

    def override_system_main(self, text: str) -> "PromptSet":
        """返回一个 system_main 被覆盖的新 PromptSet。"""
        return PromptSet(
            system_main=text,
            compaction_system=self.compaction_system,
            compaction_initial=self.compaction_initial,
            compaction_update=self.compaction_update,
            compaction_turn_prefix=self.compaction_turn_prefix,
            tool_guidelines=dict(self.tool_guidelines),
            extra=dict(self.extra),
        )


def load_prompt(path: str | Path) -> str:
    """从文件加载单个提示词。"""
    p = Path(path)
    if not p.exists():
        raise FileNotFoundError(f"Prompt file not found: {p}")
    return p.read_text(encoding="utf-8")


def load_prompt_set(
    *,
    system_main: str | Path | None = None,
    compaction_system: str | Path | None = None,
    compaction_initial: str | Path | None = None,
    compaction_update: str | Path | None = None,
    compaction_turn_prefix: str | Path | None = None,
    tool_guidelines_dir: str | Path | None = None,
    extra: dict[str, str | Path] | None = None,
) -> PromptSet:
    """从外部文件加载提示词集合。

    所有路径参数为 None 时使用 Rust 内置默认值。
    提供路径时从该文件加载内容。
    """
    ps = PromptSet()

    if system_main is not None:
        ps.system_main = load_prompt(system_main)

    if compaction_system is not None:
        ps.compaction_system = load_prompt(compaction_system)

    if compaction_initial is not None:
        ps.compaction_initial = load_prompt(compaction_initial)

    if compaction_update is not None:
        ps.compaction_update = load_prompt(compaction_update)

    if compaction_turn_prefix is not None:
        ps.compaction_turn_prefix = load_prompt(compaction_turn_prefix)

    if tool_guidelines_dir is not None:
        d = Path(tool_guidelines_dir)
        if d.is_dir():
            for f in d.iterdir():
                if f.suffix == ".md":
                    ps.tool_guidelines[f.stem] = f.read_text(encoding="utf-8")

    if extra:
        for key, path in extra.items():
            ps.extra[key] = load_prompt(path)

    return ps
