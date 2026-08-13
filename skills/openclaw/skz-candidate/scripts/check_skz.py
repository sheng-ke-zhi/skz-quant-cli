#!/usr/bin/env python3
"""Print the installed skz CLI and skill contract versions as JSON."""

import subprocess


if __name__ == "__main__":
    subprocess.run(["skz", "--version"], check=True)
