from pathlib import Path


def register(ctx) -> None:
    root = Path(__file__).parent
    ctx.register_skill("skz-factor", root / "skills" / "skz-factor" / "SKILL.md", "SKZ factor")
    ctx.register_skill("skz-candidate", root / "skills" / "skz-candidate" / "SKILL.md", "SKZ candidate")
    ctx.register_skill("skz-strategy", root / "skills" / "skz-strategy" / "SKILL.md", "SKZ strategy")
    ctx.register_skill("skz-guide", root / "skills" / "skz-guide" / "SKILL.md", "SKZ guide")
    ctx.register_skill("skz-create-problem", root / "skills" / "skz-create-problem" / "SKILL.md", "SKZ create-problem")
    ctx.register_skill("skz-portfolio", root / "skills" / "skz-portfolio" / "SKILL.md", "SKZ portfolio")
