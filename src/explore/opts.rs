#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExtraOption {
    Render,
    Extract,
    Web,
}

impl ExtraOption {
    pub(super) fn all() -> Vec<Self> {
        use ExtraOption::*;
        vec![Render, Extract, Web]
    }
}

impl std::fmt::Display for ExtraOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ExtraOption::Render => "Print inscription to terminal",
                ExtraOption::Extract => "Extract inscriptions to current directory",
                ExtraOption::Web => "Open inscription on web",
            }
        )
    }
}

pub(super) struct ExtraOptions {
    pub(super) render: bool,
    pub(super) extract: bool,
    pub(super) web: bool,
}

impl ExtraOptions {
    pub(super) fn is_set(&self, opt: &ExtraOption) -> bool {
        match opt {
            ExtraOption::Render => self.render,
            ExtraOption::Extract => self.extract,
            ExtraOption::Web => self.web,
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
        self.render = false;
        self.extract = false;
        self.web = false;
    }

    pub(super) fn set_opts(&mut self, opts: &[ExtraOption]) {
        self.set_false();
        for opt in opts {
            match opt {
                ExtraOption::Render => self.render = true,
                ExtraOption::Extract => self.extract = true,
                ExtraOption::Web => self.web = true,
            }
        }
    }
}

impl Default for ExtraOptions {
    fn default() -> Self {
        Self {
            render: true,
            extract: Default::default(),
            web: Default::default(),
        }
    }
}
