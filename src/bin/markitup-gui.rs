use eframe::{egui};
use markitup;
use std::path::PathBuf;
use rfd::FileDialog;
use pulldown_cmark::{Parser,Options};
use egui_commonmark::CommonMarkViewer;
use std::thread;
use std::sync::{Arc,Mutex};
use crossbeam_channel::{unbounded, Sender, Receiver}; // 引入 crossbeam_channel
use regex::Regex;

#[derive(Debug,PartialEq,Clone)]
enum ConvertState{
    Idle,
    Converting(String),
    Down(String),
    Error(String),
}


impl Default for ConvertState{
    fn default() -> Self{
        ConvertState:: Idle
    }
}

#[derive(Debug)]
enum RightPanelMode{
    Preview,
    Editor,
}

impl Default for RightPanelMode{
    fn default()->Self{
        RightPanelMode::Preview
    }
}
enum WorkerMessage {
    ConversionResult {
        full_markdown: String,   // 完整的 Markdown 内容
        display_markdown: String, // 经过 Base64 替换后的 Markdown 内容，用于编辑器显示
    },
    Error(String), // 转换过程中发生的错误
}

fn replace_base64_in_markdown(markdown:&str) ->String{
    let re = Regex::new(r"\((data:image/[^;]+;base64,[^)]+)\)").unwrap();
    re.replace_all(markdown, "(base64_image_placeholder)").into_owned()
}

pub struct UIFramework{
    show_config_panel:bool,
    show_help_panel:bool,
    
    file_list: Vec<PathBuf>,
    select_file_path: Option<PathBuf>,
    current_markdown_content: String,
    right_panel_mode: RightPanelMode,
    markdown_cache:egui_commonmark::CommonMarkCache,

    //window sytle
    pub font_size_heading :f32,
    pub font_size_body:f32,
    pub background_color: egui::Color32,
    pub text_color: egui::Color32,
    
    //convert state
    convert_state: Arc<Mutex<ConvertState>>,
    pub egui_ctx: egui::Context,
    pub worker_sender: Sender<WorkerMessage>,   // 发送给工作线程 (通常不会从UI发送，但Default需要初始化)
    pub worker_receiver: Receiver<WorkerMessage>,
}
impl Default for UIFramework{

    
    fn default()->Self{
        let (tx, rx) = unbounded();
        Self{
            show_config_panel:false,
            show_help_panel:false,

            file_list:Vec::new(),
            select_file_path:None,
            current_markdown_content: String::new(),
            right_panel_mode: RightPanelMode::default(),
            markdown_cache: egui_commonmark::CommonMarkCache::default(),

            font_size_heading:25.0,
            font_size_body:18.0,
            background_color:egui::Color32::from_rgb(27, 27, 27),
            text_color: egui::Color32::WHITE,
            convert_state: Arc::new(Mutex::new(ConvertState::Idle)), 
            egui_ctx: egui::Context::default(),

            worker_sender: tx,
            worker_receiver: rx,
        }

    }
}

impl eframe::App for UIFramework{
    fn update(&mut self, ctx: &egui::Context, _frame:&mut eframe::Frame){
        egui::TopBottomPanel::top("top_panel").show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP),|ui|{
                    ui.menu_button("file",|ui_file_menu|{
                        if ui_file_menu.button("Open").clicked(){
                           self.open_files_dialog(); 
                        }
                    });

                    if ui.button("config").clicked() {
                        println!("点击了 config 按钮");
                        self.show_config_panel=!self.show_config_panel;
                        self.show_help_panel=false;
                    }
                    if ui.button("help").clicked() {
                        println!("点击了 help 按钮");
                        self.show_help_panel=!self.show_help_panel;
                        self.show_config_panel=false;
                    }
                });//left_to_right end
                    
  
             });//horizontal end
        });//topbottom end
        egui::CentralPanel::default().show(ctx,|ui|{
            egui::SidePanel::left("file_list").exact_width(200.0).show_inside(ui,|ui|{
                ui.vertical_centered(|ui| { // 让按钮居中
                    ui.add_space(10.0); // 顶部间距
                    ui.heading("file list");
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let file_list_guard = &self.file_list;
                        if file_list_guard.is_empty() {
                            ui.label("Please add files.");
                        } else {
                        let mut selected_idx = -1; // 用来记录哪个文件被选中
                            for (idx, path_buf) in self.file_list.iter().enumerate() {
                                let file_name = path_buf.file_name().unwrap_or_default().to_string_lossy();
                                let is_selected = self.select_file_path.as_ref() == Some(path_buf);

                                let response = ui.selectable_value(&mut is_selected.clone(), true, file_name);

                                if response.clicked() {
                                    self.select_file_path = Some(path_buf.clone());
                                    if let Some(path) = &self.select_file_path {
                                        if let Some(path_str) = path.to_str(){
                                            dbg!(path_str);
                                            let content= markitup::convert_from_path(path_str);
                                            match content{
                                                Ok(content)=>{
                                                    self.current_markdown_content=content;
                                                    self.right_panel_mode=RightPanelMode::Preview;
                                                }
                                                Err(content)=>{
                                                    eprintln!("error convert: {}", content);
                                                }
                                            }
                                        }
                                        else {
                                            eprintln!("invalid file path!{}",path.display());
                                        }
                                        
                                    }
                                    else{
                                       println!("Please select files!") // add new ui??
                                    }
                                    
                                }
                            }//end for 
                        }//end else
                        }); // end scroll area;

                    });//end vertical_centered
            });//end left side panel
            egui::CentralPanel::default().show_inside(ui,|ui|{
                ui.vertical(|ui|{
                    ui.heading(match self.right_panel_mode{
                        RightPanelMode::Preview => "Markdown preview",
                        RightPanelMode::Editor=>"Markdown Editor(Source Code)",
                    });
                    ui.add_space(5.0);

                    ui.horizontal(|ui|{
                        if ui
                            .button(match self.right_panel_mode {
                                RightPanelMode::Preview => "Change to Editor Mode",
                                RightPanelMode::Editor => "Change to Preview Mode",
                            })
                            .clicked()
                        {
                            self.right_panel_mode = match self.right_panel_mode {
                                RightPanelMode::Preview => RightPanelMode::Editor,
                                RightPanelMode::Editor => RightPanelMode::Preview,
                            };
                            println!("当前模式: {:?}", self.right_panel_mode);
                        }
                        ui.add_space(10.0); // 按钮之间的间距
                        if ui.button("Save Markdown").clicked(){
                            self.save_markdown_content();
                        }
                    });//end horizontal
                    ui.separator();
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical().show(ui,|ui|{
                        match self.right_panel_mode{
                            RightPanelMode::Preview =>{
                                let viewer = CommonMarkViewer::new("markdown_viewer_unique_id");
                                viewer.show(ui, &mut self.markdown_cache, &self.current_markdown_content);
                            }
                            RightPanelMode::Editor =>{
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.current_markdown_content)
                                        .desired_width(f32::INFINITY) // 宽度填充可用空间
                                        .desired_rows(20) // 默认高度（行数）
                                      );
                            }
                        }
                    });//end scrollarea

                });//end vertical

            });//end central panel

       }); //end central panel

        if self.show_config_panel{
        egui::Window::new("config")
            .open(&mut self.show_config_panel)
            .show(ctx,|ui|{
                ui.heading("config");
                ui.add_space(10.0);
            });
        }
        if self.show_help_panel{
            egui::Window::new("help")
                .open(&mut self.show_help_panel)
                .show(ctx,|ui|{
                    ui.heading("help");
                    ui.add_space(10.0);
                });
        }

    }
}
pub fn createFrame(){
    let app_name = "Markitup"; 

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 1200.0]) // 初始窗口大小
            .with_min_inner_size([300.0, 200.0]) // 最小窗口大小
            .with_title(app_name),
        vsync:true,
        multisampling:4,
        ..Default::default()

    };
    let app=UIFramework::default();
    eframe::run_native(
        app_name,
        native_options,
        Box::new(|cc| Box::new(UIFramework::new(cc))),
        );
}
impl UIFramework{
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default(); 
        app.egui_ctx = cc.egui_ctx.clone();
        let mut fonts= egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_custom_font".to_owned(), // Give your font a unique name within egui
            egui::FontData::from_static(include_bytes!("../../font.ttf")), // Adjust path as needed
        );
            fonts.families.get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "my_custom_font".to_owned());
            fonts.families.get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "my_custom_font".to_owned());
        cc.egui_ctx.set_fonts(fonts);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(app.font_size_heading)); // 使用标题字号作为按钮字号
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(app.font_size_body));
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(app.font_size_heading));

        // 设置颜色
        style.visuals.window_fill = app.background_color;
        style.visuals.panel_fill = app.background_color;
        //style.visuals.text_color = app.text_color; // 默认文本颜色

        cc.egui_ctx.set_style(style);

        cc.egui_ctx.set_pixels_per_point(1.2);
        app
    }

    fn open_files_dialog(&mut self) {
        let result = FileDialog::new()
            .set_title("Select files")
            .add_filter("All Files", &["*"])
            .pick_files(); // This call is blocking

        if let Some(paths) = result {
            for path_buf in paths {
                if !self.file_list.contains(&path_buf) { // Avoid duplicates
                    self.file_list.push(path_buf.clone());
                    println!("Added file: {:?}", path_buf);
                }
            }
        } else {
            println!("File selection canceled");
        }
    }
    fn save_markdown_content(&self){
        if let Some(ref selected_path) = self.select_file_path {
            // 建议保存为 .md 文件，并尝试使用原始文件的目录和文件名
            let default_save_path = selected_path.with_extension("md");
            let current_dir_path = PathBuf::from(".");
            let file_dialog_result = FileDialog::new()
                .set_title("另存为 Markdown...")
                .add_filter("Markdown 文件", &["md"])
                // 设置默认目录为当前选定文件的父目录，如果文件没有父目录，则使用当前工作目录
                .set_directory(&current_dir_path)
                .set_file_name(default_save_path.file_name().unwrap_or_default().to_string_lossy())
                .save_file(); // 这会阻塞当前线程直到用户选择或取消

            if let Some(save_path) = file_dialog_result {
                match std::fs::write(&save_path, &self.current_markdown_content) {
                    Ok(_) => println!("Markdown 已成功保存到: {:?}", save_path),
                    Err(e) => eprintln!("保存 Markdown 失败: {}", e),
                }
            } else {
                println!("保存操作已取消。");
            }
        } else {
            println!("没有文件被选中，无法保存内容。");
            // add ui?
        }
    }
    pub fn load_and_set_markdown_content(&mut self, path_buf: &PathBuf) {
        self.select_file_path = Some(path_buf.clone());
        let file_name_str = path_buf.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned();

        // 2. 将转换状态设置为 "Converting"，以便 UI 可以显示加载提示
        *self.convert_state.lock().unwrap() = ConvertState::Converting(file_name_str.clone());

        // 3. 克隆必要的变量以发送到新线程
        let ui_ctx = self.egui_ctx.clone(); // egui context 用于请求 UI 重绘
        let convert_state_arc = Arc::clone(&self.convert_state); // 共享转换状态
        let path_for_thread = path_buf.clone(); // 要转换的文件路径
        let sender_for_thread = self.worker_sender.clone(); // 用于将结果发送回主线程

        // 4. 启动一个新线程来执行耗时操作
        thread::spawn(move || {
            // 尝试将 PathBuf 转换为 &str，如果失败则返回错误
            let result = if let Some(path_str) = path_for_thread.to_str() {
                // 调用您的 markitup 库进行转换
                markitup::convert_from_path(path_str)
            } else {
                Err(format!("文件路径包含无效的 UTF-8 字符: {}", path_for_thread.display()))
            };

            match result {
                Ok(full_markdown_content) => {
                    let display_content = replace_base64_in_markdown(&full_markdown_content);
                    sender_for_thread.send(WorkerMessage::ConversionResult {
                        full_markdown: full_markdown_content,
                        display_markdown: display_content,
                    }).unwrap(); 
                },
                Err(e) => {
                    sender_for_thread.send(WorkerMessage::Error(format!("转换文件 '{}' 失败: {}", file_name_str, e))).unwrap();
                },
            }
            ui_ctx.request_repaint();
        });
    }
    
}

fn main(){
    createFrame();

}
