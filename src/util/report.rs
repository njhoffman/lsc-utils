//! `--report` summary counter.
//!
//! Tallies folder/recognized-file/unrecognized-file counts during rendering
//! and produces either the short or long colorls report block.

use crate::config::IconKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Short,
    Long,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReportCounts {
    pub folders: u64,
    pub recognized_files: u64,
    pub unrecognized_files: u64,
}

impl ReportCounts {
    pub fn record(&mut self, kind: IconKind) {
        match kind {
            IconKind::Folder | IconKind::DefaultFolder => self.folders += 1,
            IconKind::File => self.recognized_files += 1,
            IconKind::DefaultFile => self.unrecognized_files += 1,
        }
    }

    pub fn render(&self, kind: ReportKind) -> String {
        match kind {
            ReportKind::Short => format!(
                "\n    Folders: {}, Files: {}.\n",
                self.folders,
                self.recognized_files + self.unrecognized_files
            ),
            ReportKind::Long => format!(
                "\n      Found {} items in total.\n\n\
                 \tFolders\t\t\t: {}\n\
                 \tRecognized files\t: {}\n\
                 \tUnrecognized files\t: {}\n",
                self.folders + self.recognized_files + self.unrecognized_files,
                self.folders,
                self.recognized_files,
                self.unrecognized_files,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_counts_by_kind() {
        let mut c = ReportCounts::default();
        c.record(IconKind::Folder);
        c.record(IconKind::DefaultFolder);
        c.record(IconKind::File);
        c.record(IconKind::DefaultFile);
        c.record(IconKind::DefaultFile);
        assert_eq!(c.folders, 2);
        assert_eq!(c.recognized_files, 1);
        assert_eq!(c.unrecognized_files, 2);
    }

    #[test]
    fn short_report() {
        let c = ReportCounts {
            folders: 3,
            recognized_files: 5,
            unrecognized_files: 2,
        };
        let s = c.render(ReportKind::Short);
        assert!(s.contains("Folders: 3, Files: 7."));
    }

    #[test]
    fn long_report_lists_all_categories() {
        let c = ReportCounts {
            folders: 1,
            recognized_files: 2,
            unrecognized_files: 3,
        };
        let s = c.render(ReportKind::Long);
        assert!(s.contains("Found 6 items"));
        assert!(s.contains("Folders\t\t\t: 1"));
        assert!(s.contains("Recognized files\t: 2"));
        assert!(s.contains("Unrecognized files\t: 3"));
    }
}
