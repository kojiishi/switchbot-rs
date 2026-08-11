import argparse
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass
from enum import Enum, auto
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any


class Section(Enum):
    NONE = auto()
    DEVICE_LIST_INFO = auto()
    CONTROL_COMMANDS = auto()


@dataclass
class HelpData:
    commands: dict[str, list[dict[str, Any]]]
    commands_ir: dict[str, list[dict[str, Any]]]
    device_type_aliases: dict[str, list[str]]

    def __init__(self, devices_dir: str | Path) -> None:
        self.commands = {}
        self.commands_ir = {}
        self.device_type_aliases = {}

        devices_path = Path(devices_dir)
        if not devices_path.exists():
            print(f"Error: Directory {devices_dir} does not exist.", file=sys.stderr)
            sys.exit(1)

        for md_file in sorted(devices_path.rglob("*.md")):
            parsed = ParsedMarkdown(md_file)
            if not parsed.device_name:
                continue
            print(
                f"Parsing: {md_file.relative_to(devices_path)} -> '{parsed.device_name}'"
            )

            # Add aliases using the existing add_device_type_alias function
            for alias in parsed.aliases:
                self.add_device_type_alias(alias, parsed.device_name)

            # Add commands
            for dev_type, cmd in parsed.commands:
                self.commands.setdefault(dev_type, []).append(cmd)

            # Add commands_ir (and split comma-separated device types)
            for dev_type, cmd in parsed.commands_ir:
                if (
                    dev_type == "All home appliance types except Others"
                    or dev_type == "Others"
                ):
                    # Do not split special keys
                    self.commands_ir.setdefault(dev_type, []).append(cmd)
                else:
                    # Split comma-separated names
                    names = [n.strip() for n in dev_type.split(",")]
                    for name in names:
                        self.commands_ir.setdefault(name, []).append(cmd)

        # Post-processing / Finalization
        # 1. Add "Standing Fan" -> "Standing Circulator Fan" alias
        # This is due to an inconsistency in the official SwitchBot API documentation
        # where webhook events report the type as "Standing Fan" but control commands
        # are under "Standing Circulator Fan".
        self.add_device_type_alias("Standing Fan", "Standing Circulator Fan")

        # 2. commands_ir finalization (Others and All home appliance types except Others)
        other_key = "Others"
        if other_key in self.commands_ir:
            others = self.commands_ir.pop(other_key)
            # Append others to all other IR device types
            for k, v in self.commands_ir.items():
                # Avoid modifying shared dict references if they were shared
                v.extend([json.loads(json.dumps(x)) for x in others])

        all_key = "All home appliance types except Others"
        if all_key in self.commands_ir:
            all_app = self.commands_ir.pop(all_key)
            # Prepend to all other IR device types
            for k, v in self.commands_ir.items():
                copied = [json.loads(json.dumps(x)) for x in all_app]
                self.commands_ir[k] = copied + v

    def add_device_type_alias(self, alias: str, target: str) -> None:
        if alias not in self.device_type_aliases:
            self.device_type_aliases[alias] = [target]
        elif target not in self.device_type_aliases[alias]:
            self.device_type_aliases[alias].append(target)

    @staticmethod
    def get_devices_dir(
        devices_dir: str | Path,
    ) -> tuple[str | Path, TemporaryDirectory[str]]:
        if devices_dir:
            return (devices_dir, None)
        temp_dir = tempfile.TemporaryDirectory()
        clone_dir = Path(temp_dir.name) / "repo"
        print("Cloning https://github.com/OpenWonderLabs/SwitchBotAPI.git...")
        try:
            subprocess.run(
                [
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/OpenWonderLabs/SwitchBotAPI.git",
                    str(clone_dir),
                ],
                check=True,
                capture_output=True,
                text=True,
            )
        except subprocess.CalledProcessError as e:
            print(f"Error: Failed to clone repository: {e.stderr}", file=sys.stderr)
            sys.exit(1)

        devices_dir = clone_dir / "devices"
        if not devices_dir.exists():
            print(
                "Error: 'devices' directory not found in the cloned repository.",
                file=sys.stderr,
            )
            sys.exit(1)
        return (devices_dir, temp_dir)


class ParsedMarkdown:
    device_name: str
    aliases: list[str]
    commands: list[tuple[str, dict[str, Any]]]
    commands_ir: list[tuple[str, dict[str, Any]]]
    _section: Section
    _in_command_table: bool
    _current_device_type: str

    def __init__(self, file_path: Path) -> None:
        self.device_name = ""
        self.aliases = []
        self.commands = []
        self.commands_ir = []
        self._section = Section.NONE
        self._in_command_table = False
        self._current_device_type = ""

        with open(file_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
        for line in lines:
            stripped = line.strip()

            # Section updates
            if stripped == "## Device List Information":
                self._section = Section.DEVICE_LIST_INFO
                self._in_command_table = False
                continue
            elif stripped == "## Control Commands":
                self._section = Section.CONTROL_COMMANDS
                self._in_command_table = False
                continue
            elif stripped.startswith(("## ", "# ")):
                if stripped.startswith("# ") and not self.device_name:
                    self.device_name = stripped[2:].strip()
                self._section = Section.NONE
                self._in_command_table = False
                continue

            if self._section == Section.DEVICE_LIST_INFO:
                self._parse_device_list_info_line(stripped)
            elif self._section == Section.CONTROL_COMMANDS:
                self._parse_control_commands_line(stripped, file_path.name)

    def _parse_device_list_info_line(self, stripped: str) -> None:
        if not (stripped.startswith("|") and stripped.endswith("|")):
            return

        columns = [c.strip() for c in stripped.strip("|").split("|")]
        if len(columns) < 3:
            return
        key = columns[0]
        description = columns[2]
        if key != "deviceType":
            return

        # Extract italicized/bolded names
        matches = re.findall(r"[*_]([^*_]+)[*_]", description)
        for m in matches:
            m = m.strip()
            if m and m != self.device_name:
                self.aliases.append(m)

    def _parse_control_commands_line(self, stripped: str, file_name: str) -> None:
        if not (stripped.startswith("|") and stripped.endswith("|")):
            self._in_command_table = False
            return

        columns = [c.strip() for c in stripped.strip("|").split("|")]
        if not self._in_command_table:
            if len(columns) >= 5 and columns[0] == "deviceType":
                self._in_command_table = True
            return
        if columns[0].startswith("-"):
            return
        if columns[0]:
            self._current_device_type = columns[0]
        if not self._current_device_type:
            return

        command_type = columns[1].strip("`")
        command = columns[2]
        parameter = columns[3]
        description = columns[4]
        if not command:
            return

        cmd_help = {
            "command": {
                "command": command,
                "parameter": parameter,
                "commandType": command_type,
            },
            "description": description,
        }
        # Check if the file is for virtual infrared remote devices
        if file_name == "virtual-infrared-remote-devices.md":
            self.commands_ir.append((self._current_device_type, cmd_help))
        else:
            self.commands.append((self._current_device_type, cmd_help))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate SwitchBotAPI help JSON from Markdown files."
    )
    parser.add_argument(
        "devices_dir",
        nargs="?",
        help="Path to the SwitchBotAPI devices directory containing md files",
    )
    parser.add_argument(
        "-o", "--output", help="Path to the output JSON file", default="help_data.json"
    )
    args = parser.parse_args()
    devices_dir, temp_dir = HelpData.get_devices_dir(args.devices_dir)

    try:
        data = HelpData(devices_dir)
    finally:
        if temp_dir:
            temp_dir.cleanup()

    output_path: Path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(asdict(data), f, indent=2, ensure_ascii=False)

    print(f"Successfully generated JSON at: {output_path.resolve()}")


if __name__ == "__main__":
    main()
