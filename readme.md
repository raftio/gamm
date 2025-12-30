# gamm 

**gamm** (Git Account Manager) automatically switches your git identity based on which repository you're working in. Perfect for developers juggling multiple git accounts—like personal and work profiles—who want to ensure commits are always made with the correct name, email, and GPG settings.

### How It Works

1. **Create profiles** with different git credentials (name, email, GPG signing preferences)
2. **Assign profiles to repositories** — when you first commit to a new repo, gamm prompts you to pick a profile
3. **Automatic switching** — on every commit, a pre-commit hook detects the repository and applies the correct git config globally before the commit proceeds

```
Git Account Manager - Manage multiple git configurations

Usage: gamm <COMMAND>

Commands:
  version     Display version information
  init        Initialize gamm and install git hooks
  cleanup     Remove gamm git hooks
  pre-commit  Pre-commit hook: apply git config based on repository URL
  repo        Manage repository configurations
  profile     Manage profile configurations
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## Install 

```bash
cargo install --git https://github.com/raftio/gam.git
```

## Getting Started

1. **Initialize gamm** to install git hooks globally:

```bash
gamm init
```

2. **Create a profile** - when you first commit in a repository, gamm will prompt you to create a profile with your git credentials (name and email).

3. **Manage profiles**:

```bash
# List all profiles
gamm profile list

# Delete a profile
gamm profile delete
```

4. **Manage repositories**:

```bash
# List all configured repositories
gamm repo list

# Delete a repository configuration
gamm repo delete
```

## LICENSE

This project is licensed under the [MIT License](LICENSE).