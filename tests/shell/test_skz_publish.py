import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest


BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
SNIPPET = REPO / 'scripts/shell/skz-publish.zsh'
INSTALLER = REPO / 'scripts/install_skz_publish.sh'
SHELLS = ['/bin/bash', '/bin/zsh']


class PublishTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(dir=BASE)
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.results = self.root / 'results with spaces'
        self.results.mkdir()
        self.bin_dir = self.root / 'bin'
        self.bin_dir.mkdir()
        self.calls_file = self.root / 'calls.jsonl'
        mock = self.bin_dir / 'skz'
        mock.write_text(
            '#!/usr/bin/env python3\n'
            'import json, os, sys\n'
            'with open(os.environ["MOCK_CALLS"], "a") as target:\n'
            '    target.write(json.dumps(sys.argv[1:]) + "\\n")\n'
            'print(json.dumps({"promotion_id": "mock-id"}))\n'
            'sys.exit(int(os.environ.get("MOCK_EXIT", "0")))\n'
        )
        mock.chmod(0o755)
        self.env = dict(os.environ, PATH=str(self.bin_dir) + ':' + os.environ['PATH'],
                        MOCK_CALLS=str(self.calls_file))

    def candidate(self, relative='run123/S.toml', directory=False):
        target = self.results / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if directory:
            target.mkdir()
        else:
            target.touch()

    def invoke(self, shell, arguments, tail='', extra_env=None, prefix=''):
        self.calls_file.write_text('')
        script = prefix + 'source ' + shlex.quote(str(SNIPPET)) + '\n'
        script += 'skz-publish ' + shlex.join(arguments) + '\n' + tail
        environment = dict(self.env, **(extra_env or {}))
        result = subprocess.run([shell, '-c', script], env=environment,
                                capture_output=True, text=True, timeout=10)
        calls = [json.loads(line) for line in self.calls_file.read_text().splitlines()]
        return result, calls

    def assert_rejected(self, arguments):
        for shell in SHELLS:
            with self.subTest(shell=shell, arguments=arguments):
                result, calls = self.invoke(shell, arguments)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(calls, [])

    def test_single_match(self):
        self.candidate()
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['strategy', 'register', str(self.results / 'run123/S.toml')]])
                self.assertNotIn('[ok]', result.stdout)

    def test_multiple_different_files_are_rejected(self):
        self.candidate('9/S.toml')
        self.candidate('10/S.toml')
        (self.results / '9/S.toml').write_text('strategy = "S"\npost_process = "A"\n')
        (self.results / '10/S.toml').write_text('strategy = "S"\npost_process = "B"\n')
        self.assert_rejected([str(self.results), 'S'])

    def test_identical_duplicates_are_registered_once_without_run_id(self):
        self.candidate('9/strategies/problem/route/S.toml')
        self.candidate('10/backtest_results/S/S.toml')
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(len(calls), 1)
                self.assertEqual(calls[0][:2], ['strategy', 'register'])
                self.assertIn(Path(calls[0][2]), [self.results / '9/strategies/problem/route/S.toml',
                                                self.results / '10/backtest_results/S/S.toml'])

    def test_explicit_run_selects_only_requested_candidate(self):
        self.candidate('9/S.toml')
        self.candidate('10/S.toml')
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S', '--run-id', '10'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['promote', 'start', '10', 'S']])

    def test_unknown_explicit_run_is_rejected(self):
        self.candidate()
        self.assert_rejected([str(self.results), 'S', '--run-id', 'missing'])

    def test_nested_files_are_registered_without_guessing_run_id(self):
        self.candidate('run123/deeper/S.toml')
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['strategy', 'register', str(self.results / 'run123/deeper/S.toml')]])

    def test_root_level_configuration_is_registered(self):
        self.candidate('S.toml')
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['strategy', 'register', str(self.results / 'S.toml')]])

    def test_directories_are_not_candidates(self):
        self.candidate(directory=True)
        self.assert_rejected([str(self.results), 'S'])

    def test_dot_runs_is_ignored(self):
        self.candidate('.runs/S.toml')
        self.candidate('.runs/run123/S.toml')
        self.assert_rejected([str(self.results), 'S'])

    def test_pattern_characters_are_rejected(self):
        self.candidate('run123/REAL.toml')
        for code in ['*', '?', '[REAL]', '../REAL']:
            self.assert_rejected([str(self.results), code])

    def test_missing_or_empty_memo_returns_without_exiting_shell(self):
        self.candidate()
        for shell in SHELLS:
            for suffix in [['--memo'], ['--memo', '']]:
                with self.subTest(shell=shell, suffix=suffix):
                    result, calls = self.invoke(shell, [str(self.results), 'S'] + suffix,
                                                'printf "STATUS:%s\\nAFTER_CALL\\n" "$?"\n')
                    self.assertEqual(result.returncode, 0)
                    self.assertIn('STATUS:2\nAFTER_CALL', result.stdout)
                    self.assertEqual(calls, [])

    def test_memo_with_spaces_is_one_argument(self):
        self.candidate()
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S', '--run-id', 'run123', '-m', 'note with spaces'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['promote', 'start', 'run123', 'S', '--memo', 'note with spaces']])

    def test_missing_or_invalid_run_id_is_rejected(self):
        self.candidate()
        for suffix in [['--run-id'], ['--run-id', ''], ['--run-id', '../run123'],
                       ['--run-id', '.runs'], ['--run-id', '*']]:
            self.assert_rejected([str(self.results), 'S'] + suffix)

    def test_cli_failure_is_not_retried(self):
        self.candidate()
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'], extra_env={'MOCK_EXIT': '5'})
                self.assertEqual(result.returncode, 5)
                self.assertEqual(len(calls), 1)

    def test_nounset_shell_supports_no_memo(self):
        self.candidate()
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'], prefix='set -u\n')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['strategy', 'register', str(self.results / 'run123/S.toml')]])

    def test_memo_without_platform_run_id_is_rejected_before_any_write(self):
        self.candidate()
        self.assert_rejected([str(self.results), 'S', '--memo', 'note'])

    def test_differences_in_whitespace_are_not_deduplicated(self):
        self.candidate('9/S.toml')
        self.candidate('10/S.toml')
        (self.results / '9/S.toml').write_text('strategy="S"\n')
        (self.results / '10/S.toml').write_text('strategy = "S"\n')
        self.assert_rejected([str(self.results), 'S'])

    def test_hidden_directories_are_searched_but_dot_runs_is_pruned(self):
        self.candidate('.results/deep/S.toml')
        self.candidate('.runs/S.toml')
        (self.results / '.runs/S.toml').write_text('different')
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, [str(self.results), 'S'])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, [['strategy', 'register', str(self.results / '.results/deep/S.toml')]])

    def test_symbolic_link_directory_is_not_followed(self):
        outside = self.root / 'outside'
        outside.mkdir()
        (outside / 'S.toml').write_text('outside')
        (self.results / 'linked').symlink_to(outside, target_is_directory=True)
        self.assert_rejected([str(self.results), 'S'])

    def test_help_explains_register_default(self):
        for shell in SHELLS:
            with self.subTest(shell=shell):
                result, calls = self.invoke(shell, ['--help'])
                self.assertEqual(result.returncode, 0)
                self.assertIn('strategy register', result.stdout)
                self.assertEqual(calls, [])

    def test_symlink_candidate_is_ignored(self):
        self.candidate('run123/actual.toml')
        (self.results / 'run123/S.toml').symlink_to('actual.toml')
        self.assert_rejected([str(self.results), 'S'])

    def test_scan_failure_is_not_submitted(self):
        self.candidate()
        broken_find = self.bin_dir / 'find'
        broken_find.write_text('#!/bin/sh\nprintf "%s\\0" "$1/run123"\nexit 1\n')
        broken_find.chmod(0o755)
        self.assert_rejected([str(self.results), 'S'])

    def test_comparison_failure_is_not_submitted(self):
        self.candidate('9/S.toml')
        self.candidate('10/S.toml')
        broken_cmp = self.bin_dir / 'cmp'
        broken_cmp.write_text('#!/bin/sh\nexit 2\n')
        broken_cmp.chmod(0o755)
        self.assert_rejected([str(self.results), 'S'])

    def test_unknown_option_and_extra_positionals_are_rejected(self):
        self.candidate()
        for suffix in [['--unknown'], ['extra']]:
            self.assert_rejected([str(self.results), 'S'] + suffix)


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(dir=BASE)
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.source = self.root / 'source folder' / 'snippet.zsh'
        self.source.parent.mkdir()
        self.source.write_text('loaded_by_test=yes\n')
        self.target = self.root / "target folder's snippets"
        self.home = self.root / 'home'
        self.home.mkdir()
        self.rc = self.home / '.bashrc'
        self.rc.write_text('unchanged\n')
        self.env = dict(os.environ, HOME=str(self.home))

    def install(self, *arguments, environment=None):
        return subprocess.run(['/bin/bash', str(INSTALLER), *arguments],
                              cwd=self.root, env=environment or self.env,
                              capture_output=True, text=True, timeout=10)

    def standard(self, *arguments):
        return self.install('--target-dir', str(self.target), '--source', str(self.source), *arguments)

    def test_relative_source_becomes_absolute_and_loads_elsewhere(self):
        result = self.install('--target-dir', str(self.target), '--source', 'source folder/snippet.zsh')
        self.assertEqual(result.returncode, 0, result.stderr)
        link = self.target / 'skz-publish.zsh'
        self.assertEqual(os.readlink(link), str(self.source))
        for shell in SHELLS:
            loaded = subprocess.run([shell, '-c', 'source ' + shlex.quote(str(link)) + '; echo "$loaded_by_test"'],
                                    cwd=self.home, capture_output=True, text=True)
            self.assertEqual(loaded.stdout.strip(), 'yes')

    def test_printed_source_guard_uses_actual_target_and_handles_spaces(self):
        result = self.standard()
        self.assertEqual(result.returncode, 0, result.stderr)
        guards = [line.strip() for line in result.stdout.splitlines() if line.strip().startswith('if [[')]
        self.assertEqual(len(guards), 1, result.stdout)
        guard = guards[0]
        for shell in SHELLS:
            loaded = subprocess.run([shell, '-c', guard + '\necho "$loaded_by_test"'],
                                    env=self.env, capture_output=True, text=True)
            self.assertEqual(loaded.stdout.strip(), 'yes', loaded.stderr)
        self.assertEqual(self.rc.read_text(), 'unchanged\n')

    def test_environment_target_is_used(self):
        environment = dict(self.env, SKZ_SNIPPETS_DIR=str(self.target))
        result = self.install('--source', str(self.source), environment=environment)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.target / 'skz-publish.zsh').is_file())

    def test_missing_snippet_guard_does_not_fail_shell(self):
        result = self.standard()
        guard = next(line.strip() for line in result.stdout.splitlines() if line.strip().startswith('if [['))
        self.assertEqual(self.standard('--uninstall').returncode, 0)
        for shell in SHELLS:
            loaded = subprocess.run([shell, '-c', guard], env=self.env, capture_output=True, text=True)
            self.assertEqual(loaded.returncode, 0, loaded.stderr)

    def test_default_target_is_used(self):
        environment = dict(self.env)
        environment.pop('SKZ_SNIPPETS_DIR', None)
        result = self.install('--source', str(self.source), environment=environment)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.home / 'skz/snippets/skz-publish.zsh').is_file())

    def test_repeated_install_and_uninstall_are_safe(self):
        self.assertEqual(self.standard().returncode, 0)
        self.assertEqual(self.standard().returncode, 0)
        self.assertEqual(self.standard('--uninstall').returncode, 0)
        self.assertTrue(self.source.is_file())
        self.assertFalse((self.target / 'skz-publish.zsh').is_symlink())
        self.assertEqual(self.standard('--uninstall').returncode, 0)
        self.assertEqual(self.rc.read_text(), 'unchanged\n')

    def test_regular_file_is_never_overwritten_or_deleted(self):
        self.target.mkdir()
        link = self.target / 'skz-publish.zsh'
        link.write_text('keep me')
        for arguments in [[], ['--uninstall']]:
            result = self.standard(*arguments)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(link.read_text(), 'keep me')

    def test_wrong_and_broken_links_are_explicitly_rejected(self):
        self.target.mkdir()
        link = self.target / 'skz-publish.zsh'
        link.symlink_to(self.root / 'missing.zsh')
        result = self.standard()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('符号链接', result.stderr)
        self.assertEqual(os.readlink(link), str(self.root / 'missing.zsh'))

    def test_installer_missing_option_value_returns_two(self):
        for option in ['--source', '--target-dir']:
            result = self.install(option)
            self.assertEqual(result.returncode, 2, result.stderr)


if __name__ == '__main__':
    unittest.main(verbosity=2)
