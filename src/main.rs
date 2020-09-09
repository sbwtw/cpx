use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

struct Cpx {
    copy_config: CopyConfig,
    file_config: ConfigInfo,
}

impl Cpx {
    fn new(copy_config: CopyConfig, file_config: ConfigInfo) -> Self {
        Self {
            copy_config,
            file_config,
        }
    }

    fn execute<T: AsRef<str>>(&self, tags: Option<Vec<T>>, files: Option<Vec<T>>) {
        let copy_files = self.file_config.calculate_file_list(tags, files);

        let from = self.src_path().expect("src path not found");
        let to = self.dst_path().expect("dst path not found");
        self.execute_copy(from, to, copy_files);
    }

    fn src_path(&self) -> Option<PathBuf> {
        self.copy_config
            .from
            .as_ref()
            .and_then(|x| self.file_config.path_list.get(x).map(|x| x.path.clone()))
    }

    fn dst_path(&self) -> Option<PathBuf> {
        self.copy_config
            .to
            .as_ref()
            .and_then(|x| self.file_config.path_list.get(x).map(|x| x.path.clone()))
    }

    fn execute_copy<P: AsRef<Path>>(&self, from: P, to: P, files: HashSet<FileInfo>) {
        for f in files {
            let src = from.as_ref().join(&f.relative_path);
            let dst = to.as_ref().join(f.relative_path);

            if self.copy_config.verbose > 0 || self.copy_config.dry_run {
                println!("Copy:\n{}\nto:\n{}", src.display(), dst.display());
            }

            if !self.copy_config.dry_run {
                if let Err(e) = std::fs::copy(&src, &dst) {
                    eprintln!(
                        "Copy:\n{}\nto:\n{}\nfailed, {:?}",
                        src.display(),
                        dst.display(),
                        e
                    );
                }
            }
        }
    }
}

struct CopyConfig {
    pub from: Option<String>,
    pub to: Option<String>,
    pub dry_run: bool,
    pub create_dir: bool,
    pub verbose: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PathInfo {
    path: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct TagInfo {
    file_list: Option<Vec<String>>,
    script_list: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct FileInfo {
    relative_path: PathBuf,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ScriptInfo {
    from: PathBuf,
    to: PathBuf,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ConfigInfo {
    path_list: HashMap<String, PathInfo>,
    tag_list: HashMap<String, TagInfo>,
    file_list: HashMap<String, FileInfo>,
    script_list: HashMap<String, ScriptInfo>,
}

impl ConfigInfo {
    fn calculate_file_list<T: AsRef<str>>(
        &self,
        tags: Option<Vec<T>>,
        files: Option<Vec<T>>,
    ) -> HashSet<FileInfo> {
        let mut selected_files: Vec<_> = vec![];
        if let Some(x) = tags {
            for t in x {
                if let Some(mut item) = self
                    .tag_list
                    .get(t.as_ref())
                    .and_then(|x| x.file_list.clone())
                {
                    selected_files.append(&mut item);
                }
            }
        }

        if let Some(x) = files {
            for f in x {
                selected_files.push(f.as_ref().to_string());
            }
        }

        selected_files
            .iter()
            .map(|x| {
                self.file_list
                    .get(x)
                    .expect(&format!("file {} not found in config", x))
                    .clone()
            })
            .collect()
    }
}

fn main() {
    let default_config = dirs::home_dir()
        .and_then(|x| x.join("cpx.yaml").to_str().map(|x| x.to_owned()))
        .unwrap_or("cpx.yaml".to_owned());
    let m = App::new("Help you copy files")
        .version("0.1")
        .author("sbw <sbw@sbw.so>")
        .about("Help you copy files!")
        .arg(
            Arg::with_name("spec")
                .help("specific source path and destination path")
                .takes_value(true)
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("files")
                .help("copy files")
                .long("file")
                .takes_value(true)
                .required_unless("tags")
                .multiple(true),
        )
        .arg(
            Arg::with_name("tags")
                .long("tag")
                .takes_value(true)
                .required_unless("files")
                .multiple(true),
        )
        .arg(Arg::with_name("verbose").short("v"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .takes_value(true)
                .default_value(&default_config),
        )
        .arg(Arg::with_name("dry-run").long("dry-run").help("Dry run"))
        .get_matches();

    let tags: Option<Vec<_>> = m.values_of("tags").map(|x| x.collect());
    let files: Option<Vec<_>> = m.values_of("files").map(|x| x.collect());
    let f = File::open(m.value_of("config").unwrap()).expect("File read failed!");
    let config: ConfigInfo = serde_yaml::from_reader(f).expect("File parse failed!");

    let mut cpx_config = CopyConfig {
        from: None,
        to: None,
        dry_run: m.is_present("dry-run"),
        create_dir: true,
        verbose: m.occurrences_of("verbose"),
    };

    let spec: Vec<_> = m
        .value_of("spec")
        .map(|x| x.split(':').collect())
        .expect("spec error");
    if spec.len() == 2 {
        cpx_config.from = Some(spec[0].to_owned());
        cpx_config.to = Some(spec[1].to_owned());
    }

    let cpx = Cpx::new(cpx_config, config);
    cpx.execute(tags, files);

    // let mut config = ConfigInfo {
    //     path_list: HashMap::new(),
    //     tag_list: HashMap::new(),
    //     file_list: HashMap::new(),
    //     script_list: HashMap::new(),
    // };
    // config.tag_list.insert(
    //     "aaa".to_string(),
    //     TagInfo {
    //         file_list: vec!["aaa".to_owned(), "bbb".to_owned()],
    //         script_list: vec![],
    //     },
    // );
    // config.tag_list.insert(
    //     "aaab".to_string(),
    //     TagInfo {
    //         file_list: vec![],
    //         script_list: vec![],
    //     },
    // );
    //
    // println!("{}", serde_yaml::to_string(&config).unwrap());
}
