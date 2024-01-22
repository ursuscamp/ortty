#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExtraOption {
    Extract,
    Web,
    Ordinals,
    Atomicals,
}

impl ExtraOption {
    pub(super) fn all() -> Vec<Self> {
        use ExtraOption::*;
        vec![Extract, Web, Ordinals, Atomicals]
    }
}

impl std::fmt::Display for ExtraOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ExtraOption::Extract => "Extract inscriptions to current directory",
                ExtraOption::Web => "Open inscription on web",
                ExtraOption::Ordinals => "Show standard Ordinals inscriptions",
                ExtraOption::Atomicals => "Show Atomicals inscriptions",
            }
        )
    }
}

pub(super) struct ExtraOptions {
    extract: bool,
    web: bool,
    ordinals: bool,
    atomicals: bool,
}

impl ExtraOptions {
    pub(super) fn is_set(&self, opt: &ExtraOption) -> bool {
        match opt {
            ExtraOption::Extract => self.extract,
            ExtraOption::Web => self.web,
            ExtraOption::Ordinals => self.ordinals,
            ExtraOption::Atomicals => self.atomicals,
        }
    }

    pub(super) fn current_set_indexes(&self) -> Vec<usize> {
        ExtraOption::all()
            .into_iter()
            .enumerate()
            .filter_map(|(i, o)| if self.is_set(&o) { Some(i) } else { None })
            .collect()
    }

    pub(super) fn set_false(&mut self) {
        self.extract = false;
        self.web = false;
        self.ordinals = false;
        self.atomicals = false;
    }

    pub(super) fn set_opts(&mut self, opts: &[ExtraOption]) {
        self.set_false();
        for opt in opts {
            match opt {
                ExtraOption::Extract => self.extract = true,
                ExtraOption::Web => self.web = true,
                ExtraOption::Ordinals => self.ordinals = true,
                ExtraOption::Atomicals => self.atomicals = true,
            }
        }
    }
}

impl Default for ExtraOptions {
    fn default() -> Self {
        Self {
            extract: Default::default(),
            web: Default::default(),
            ordinals: true,
            atomicals: Default::default(),
        }
    }
}
