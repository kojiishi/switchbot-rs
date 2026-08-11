import tempfile
import unittest
from pathlib import Path

from generate import HelpData, ParsedMarkdown, Section


class TestGenerate(unittest.TestCase):
    def test_section_enum(self):
        self.assertEqual(Section.NONE.name, "NONE")
        self.assertEqual(Section.DEVICE_LIST_INFO.name, "DEVICE_LIST_INFO")
        self.assertEqual(Section.CONTROL_COMMANDS.name, "CONTROL_COMMANDS")

    def test_parse_markdown_file_physical(self):
        content = """# Test Device

## Device List Information

| Key        | Value Type | Description |
| ---------- | ---------- | ----------- |
| deviceType | String     | device type. _TestAlias_ |

## Control Commands

Here is some text before the table that should not break parsing.

| deviceType  | commandType | Command      | command parameter | Description |
| ----------- | ----------- | ------------ | ----------------- | ----------- |
| Test Device | command     | turnOn       | default           | Turn on |
|             | command     | setMode      | 1                 | Set mode |
|             | custom      | specialCmd   | param1            | Special command |
|             | command     |              |                   | Ignored |
"""
        with tempfile.TemporaryDirectory() as tmpdir:
            file_path = Path(tmpdir) / "test-device.md"
            with open(file_path, "w", encoding="utf-8") as f:
                f.write(content)

            parsed = ParsedMarkdown(file_path)

            self.assertEqual(parsed.device_name, "Test Device")
            self.assertEqual(parsed.aliases, ["TestAlias"])
            self.assertEqual(len(parsed.commands_ir), 0)

            # The empty command should be ignored, leaving 3 commands
            self.assertEqual(len(parsed.commands), 3)

            # 1. turnOn
            self.assertEqual(
                parsed.commands[0],
                (
                    "Test Device",
                    {
                        "command": {
                            "command": "turnOn",
                            "parameter": "default",
                            "commandType": "command",
                        },
                        "description": "Turn on",
                    },
                ),
            )

            # 2. setMode
            self.assertEqual(
                parsed.commands[1],
                (
                    "Test Device",
                    {
                        "command": {
                            "command": "setMode",
                            "parameter": "1",
                            "commandType": "command",
                        },
                        "description": "Set mode",
                    },
                ),
            )

            # 3. specialCmd
            self.assertEqual(
                parsed.commands[2],
                (
                    "Test Device",
                    {
                        "command": {
                            "command": "specialCmd",
                            "parameter": "param1",
                            "commandType": "custom",
                        },
                        "description": "Special command",
                    },
                ),
            )

    def test_parse_markdown_file_ir(self):
        content = """# Virtual infrared remote devices

## Control Commands

| deviceType | commandType | Command | command parameter | Description |
| ---------- | ----------- | ------- | ----------------- | ----------- |
| Others     | `customize` | btn1    | default           | User button |
| TV         | command     | turnOn  | default           | TV on |
"""
        with tempfile.TemporaryDirectory() as tmpdir:
            file_path = Path(tmpdir) / "virtual-infrared-remote-devices.md"
            with open(file_path, "w", encoding="utf-8") as f:
                f.write(content)

            parsed = ParsedMarkdown(file_path)

            self.assertEqual(parsed.device_name, "Virtual infrared remote devices")
            self.assertEqual(len(parsed.commands), 0)
            self.assertEqual(len(parsed.commands_ir), 2)

            self.assertEqual(
                parsed.commands_ir[0],
                (
                    "Others",
                    {
                        "command": {
                            "command": "btn1",
                            "parameter": "default",
                            "commandType": "customize",
                        },
                        "description": "User button",
                    },
                ),
            )
            self.assertEqual(
                parsed.commands_ir[1],
                (
                    "TV",
                    {
                        "command": {
                            "command": "turnOn",
                            "parameter": "default",
                            "commandType": "command",
                        },
                        "description": "TV on",
                    },
                ),
            )

    def test_generate_devices_finalization(self):
        ir_content = """# Virtual infrared remote devices

## Control Commands

| deviceType                             | commandType | Command | command parameter | Description |
| -------------------------------------- | ----------- | ------- | ----------------- | ----------- |
| All home appliance types except Others | command     | turnOn  | default           | Turn on |
| Others                                 | `customize` | btn1    | default           | User button |
| TV                                     | command     | setCh   | 1                 | Set channel |
"""
        fan_content = """# Standing Circulator Fan

## Device List Information

| Key        | Value Type | Description |
| ---------- | ---------- | ----------- |
| deviceType | String     | device type. _Standing Circulator Fan_ |
"""
        with tempfile.TemporaryDirectory() as tmpdir:
            ir_file = Path(tmpdir) / "virtual-infrared-remote-devices.md"
            with open(ir_file, "w", encoding="utf-8") as f:
                f.write(ir_content)

            fan_file = Path(tmpdir) / "standing-circulator-fan.md"
            with open(fan_file, "w", encoding="utf-8") as f:
                f.write(fan_content)

            data = HelpData(tmpdir)

            # Check Standing Fan alias
            self.assertIn("Standing Fan", data.device_type_aliases)
            self.assertEqual(
                data.device_type_aliases["Standing Fan"], ["Standing Circulator Fan"]
            )

            # Check commands_ir finalization
            self.assertIn("TV", data.commands_ir)
            tv_cmds = data.commands_ir["TV"]

            self.assertEqual(len(tv_cmds), 4)
            self.assertEqual(tv_cmds[0]["command"]["command"], "turnOn")
            self.assertEqual(tv_cmds[1]["command"]["command"], "btn1")
            self.assertEqual(tv_cmds[2]["command"]["command"], "setCh")
            self.assertEqual(tv_cmds[3]["command"]["command"], "btn1")


if __name__ == "__main__":
    unittest.main()
