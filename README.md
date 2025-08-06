
# Filesystem Time Travel (FTT) Documentation

## Overview
**Filesystem Time Travel (FTT)** is a command-line application designed for managing and tracking changes in a filesystem.  
It allows users to:
- Take snapshots of their filesystem at different points in time.
- Rewind to previous states.
- Log snapshots.
- Compare differences between snapshots.
- Tag snapshots for easy access.

This tool is particularly useful for developers, system administrators, and anyone needing to track changes in files and directories over time.

---

## Features
- **Snapshot Management**: Create, save, and manage snapshots of the filesystem.
- **Rewind Functionality**: Restore the filesystem to a previous snapshot.
- **Diff Comparison**: Compare two snapshots and view differences in files.
- **Tagging**: Tag snapshots with meaningful labels for easier retrieval.
- **Status Checking**: Check the current status of files relative to the latest snapshot.
- **Logging**: View a log of all saved snapshots.

---

## Installation

### Prerequisites
Ensure you have **Rust** installed. You can install Rust from [rust-lang.org](https://www.rust-lang.org/).

### Build and Run

```bash
git clone <repository-url>
cd <repository-directory>
cargo build --release
./target/release/ftt
```

---

## Usage

### Command Structure

```bash
ftt <command> [options]
```

### Available Commands

#### 1. `init`
Initializes the FTT system in the specified directory.

```bash
ftt init <path>
```
- `<path>`: The directory where the FTT system will be initialized.

---

#### 2. `save`
Saves a snapshot of the current filesystem state.

```bash
ftt save <path>
```
- `<path>`: The directory to save a snapshot for.

---

#### 3. `log`
Displays the list of saved snapshots.

```bash
ftt log <path>
```
- `<path>`: The directory to check for saved snapshots.

---

#### 4. `rewind`
Rewinds the filesystem to a previous snapshot.

```bash
ftt rewind <path> [--back <number>] [--tag <tag>]
```
- `<path>`: The directory to rewind.
- `--back <number>`: Specify how many snapshots back to rewind.
- `--tag <tag>`: Specify the tag of the snapshot to rewind to.

---

#### 5. `diff`
Compares two snapshots and displays the differences.

```bash
ftt diff <path> [--from <id>] [--to <id>] [--from-tag <tag>] [--to-tag <tag>]
```
- `<path>`: The directory containing snapshots to compare.
- `--from <id>`: ID of the snapshot to compare from.
- `--to <id>`: ID of the snapshot to compare to.
- `--from-tag <tag>`: Tag of the snapshot to compare from.
- `--to-tag <tag>`: Tag of the snapshot to compare to.

---

#### 6. `tag`
Tags a snapshot with a meaningful label.

```bash
ftt tag <path> --snapshot <id> --label <label>
```
- `<path>`: The directory where the snapshot is located.
- `--snapshot <id>`: ID of the snapshot to tag.
- `--label <label>`: The label to associate with the snapshot.

---

#### 7. `status`
Checks the current status of the filesystem compared to the latest snapshot.

```bash
ftt status <path>
```
- `<path>`: The directory to check the status of.

---

## Example Usage

```bash
ftt init .                   # Initialize FTT in the current directory
ftt save .                   # Save a snapshot of the current state
ftt log .                    # View the list of saved snapshots
ftt rewind . --back 1        # Rewind to the last snapshot
ftt diff . --from 1 --to 2   # Compare the latest snapshot with the previous one
ftt tag . --snapshot 1 --label "Initial Setup"  # Tag a snapshot
ftt status .                 # Check the current status of the filesystem
```

---

## File Structure

When initialized, FTT creates a hidden directory named `.ftt` in the specified path, which contains:

- `snapshots/`: Directory for storing snapshot files.
- `blobs/`: Directory for storing file contents associated with snapshots.
- `index.json`: A JSON file containing metadata about the snapshots.
- `tags.json`: A JSON file for storing tags associated with snapshots.

---

## Error Handling

FTT provides user-friendly error messages to guide you in case of incorrect commands or issues with paths.  
Always ensure that the paths provided are valid.

---

## Contributing

Contributions are welcome! If you have suggestions or improvements, feel free to create an issue or a pull request.

---

## License

This project is licensed under the **MIT License**. See the `LICENSE` file for details.

---

## Conclusion

The **Filesystem Time Travel (FTT)** application is a powerful tool for managing filesystem snapshots.  
With its intuitive command structure and robust features, it enables users to effectively track and manage changes in their filesystems over time.
