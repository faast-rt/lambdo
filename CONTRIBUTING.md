# Contributing to FAAST

Thanks for your interest in improving the project! This document provides a guideline for general contributions to FAAST.

## Getting started

There are there general ways you can contribute to this repo:

- Proposing an enhancement or new feature
- Reporting a bug or regression
- Contributing changes to the source code

For the first two, refer to the [GitHub Issues](https://github.com/faast-rt/lambdo/issues/new/choose) which guides you through the available options along with the needed information to collect.

## Contributing changes

_Please open an issue before adding a new feature or making major changes to existing one. It allows the community to discuss the relevancy of the issue, as well as the caveats one should avoid when implementing it. Exceptions to this rule include fixing non-functional source such as code comments, documentation or other supporting files._

Proposing source code changes is done through [GitHub's standard pull request workflow](https://docs.github.com/en/get-started/quickstart/github-flow#create-a-pull-request).

### Pull Request Process

[Fork](https://docs.github.com/en/get-started/quickstart/fork-a-repo) the repository on GitHub and clone it locally.

> To setup your development environment, please refer to the [README](./README.md).

Then, create a new branch for your changes:

```bash

git checkout -b my-branch-name

```

Finally, open a pull request on the [Lambo repository](https://github.com/faast-rt/lambdo/pulls) and compare it across the `main` branch.

### Pull Request Guidelines

If your branch is a work-in-progress then please start by creating your pull requests as draft, by clicking the down-arrow next to the `Create pull request` button and instead selecting `Create draft pull request`.

This will defer the automatic process of requesting a review from the FAAST team and significantly reduces noise until you are ready. Once you are happy, you can click the `Ready for review` button.

A good pull request includes:

- A high-level description of the changes, including links to any issues that are related by adding comments like `Resolves #NNN` to your description. See [Linking a Pull Request to an Issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue) for more information.
- An up-to-date parent commit. Please make sure you are pulling in the latest `main` branch and rebasing your work on top of it, i.e. `git rebase main`.
- Unit tests where appropriate. Bug fixes will benefit from the addition of regression tests. New features will not be accepted without suitable test coverage!
- No more commits than necessary. Sometimes having multiple commits is useful for telling a story or isolating changes from one another, but please squash down any unnecessary commits that may just be for clean-up, comments or small changes.
- No additional external dependencies that aren't absolutely essential. Please do everything you can to avoid pulling in additional libraries/dependencies into `Cargo.toml` as we will be very critical of these.

### Commits conventions

Our commit convention follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)

To be more explicit, we will describe below the commit conventions message.

#### Commit Message Format

Each commit message consists of a header, a body and a footer. The header has a special format that includes a type, a scope and a subject:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Any line of the commit message cannot be longer than 100 characters! This allows the message to be easier to read on GitHub as well as in various git tools.

The footer should contain a [closing reference to an issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue) if any.

Samples:

Commit message with no body:

```
fix: add tty & fix typo on docker-compose
```

Commit message with scope:

```
feat(agent): return error message to serial
```

#### Revert

If the commit reverts a previous commit, it should begin with `revert:` , followed by the header of the reverted commit. In the body it should say: `This reverts commit <hash>.`, where the hash is the SHA of the commit being reverted.

#### Type

Must be one of the following:

- **build**: Changes that affect the build system or external dependencies (example scopes: cargo, make...)
- **chore**: Some housekeeping activity
- **ci**: Changes to our CI configuration files and scripts
- **docs**: Documentation only changes
- **feat**: A new feature
- **fix**: A bug fix
- **perf**: A code change that improves performance
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **revert**: A commit revert
- **test**: Adding missing tests or correcting existing tests

#### Scope

The scope should be the name of the related Lambdo component if applicable.

The following is the list of supported scopes:

- agent
- api
- initramfs
- shared

The scope of application is optional and only needs to be specified if the modifications concern one of the components listed above.

#### Subject

The subject contains a succinct description of the change:

- use the imperative, present tense: `change` not `changed` nor `changes`
- don't capitalize the first letter
- no dot (.) at the end

#### Body

Just as in the subject, use the imperative, present tense: `change` not `changed` nor `changes`. The body should include the motivation for the change and contrast this with previous behavior.

#### Footer

The footer should contain any information about Breaking Changes and is also the place to [reference GitHub issues that the commit closes](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue).

**Breaking Changes** should start with the word `BREAKING CHANGE:` with a space or two newlines. The rest of the commit message is then used for this.

#### Sign-off

Please consider signing the commit message at least with `Signed-Off-By`. This is a way to certify that you wrote the code or otherwise have the right to pass it on as an open-source patch. The process is simple: if you can certify the [Developer's Certificate of Origin](https://developercertificate.org/) (DCO).

To perform a sign-off with `git`, use :

```bash
git commit -s #(or --signoff)
```

## Get help

If you have questions about the contribution process, please start a [GitHub discussion](https://github.com/faast-rt/lambdo/discussions) or send your question to the contact email address: [contact@faast-rt.com](mailto://contact@faast-rt.com).
