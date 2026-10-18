use crate::{
    capture::{self, CaptureError, ErrorCode, RawFrame, geometry::crop_frame},
    services::{
        region_capture::RegionCaptureService,
        screen_capture::{ScreenCaptureService, encode_png},
        scrolling::{Progress, ScrollService, Session, Target, native},
        stitching::{Append, StitchingEngine, failure, frame_difference},
        workflow::CaptureWorkflow,
    },
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
fn main(w: &WebviewWindow) -> Result<(), CaptureError> {
    if w.label() != "main" {
        Err(failure("仅主窗口可选择长截图目标。"))
    } else {
        Ok(())
    }
}
#[tauri::command]
pub async fn scroll_targets(
    window: WebviewWindow,
    service: State<'_, Arc<ScrollService>>,
) -> Result<Vec<Target>, CaptureError> {
    main(&window)?;
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.list())
        .await
        .map_err(|_| failure("枚举窗口任务失败。"))?
}
#[tauri::command]
pub async fn scroll_prepare(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, Arc<ScrollService>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    regions: State<'_, Arc<RegionCaptureService>>,
    target_id: String,
    automatic: bool,
) -> Result<Progress, CaptureError> {
    main(&window)?;
    let target = service.target(&target_id)?;
    if automatic && !target.automatic {
        return Err(failure("此窗口未验证自动模式，请改用手动模式。"));
    }
    let t = target.clone();
    let permit = captures.try_begin()?;
    let display = tauri::async_runtime::spawn_blocking(move || {
        native::activate(&t)?;
        capture::enumerate_displays()?
            .into_iter()
            .find(|d| {
                let x = i64::from(t.bounds.x) + i64::from(t.bounds.width) / 2;
                let y = i64::from(t.bounds.y) + i64::from(t.bounds.height) / 2;
                x >= i64::from(d.x_px)
                    && x < i64::from(d.x_px) + i64::from(d.width_px)
                    && y >= i64::from(d.y_px)
                    && y < i64::from(d.y_px) + i64::from(d.height_px)
            })
            .ok_or_else(|| failure("目标显示器不可用。"))
    })
    .await
    .map_err(|_| failure("窗口准备任务失败。"))??;
    let (display, rect, _, permit) =
        super::region::select_region(window, app, permit, regions.inner().clone(), display.id)
            .await?;
    native::validate(&target)?;
    service.prepare(target, display, rect, automatic, permit)
}
#[tauri::command]
pub fn scroll_status(
    window: WebviewWindow,
    service: State<'_, Arc<ScrollService>>,
    session_id: Option<String>,
) -> Result<Option<Progress>, CaptureError> {
    if let Some(id) = session_id {
        return service.session(&id, window.label())?.status().map(Some);
    }
    main(&window)?;
    service
        .active
        .lock()
        .map_err(|_| failure("长截图状态不可用。"))?
        .as_ref()
        .map(|s| s.status())
        .transpose()
}
#[tauri::command]
pub fn scroll_action(
    window: WebviewWindow,
    service: State<'_, Arc<ScrollService>>,
    session_id: String,
    action: String,
) -> Result<(), CaptureError> {
    let s = service.session(&session_id, window.label())?;
    match action.as_str() {
        "cancel" => {
            s.action.store(2, Ordering::Release);
            if !s.started.load(Ordering::Acquire) {
                s.update("cancelled", "已取消，未保存图片。");
                s.release();
            }
        }
        "stop" => {
            if !s.started.load(Ordering::Acquire) {
                s.update("cancelled", "未开始采集，已取消。");
                s.release();
            }
            s.action.store(1, Ordering::Release);
        }
        "next" => {
            if !s.started.load(Ordering::Acquire) || s.action.load(Ordering::Acquire) != 0 {
                return Err(failure("会话尚未开始或已经结束。"));
            }
            s.next.store(true, Ordering::Release);
        }
        _ => return Err(failure("不支持的长截图操作。")),
    }
    Ok(())
}
fn sleep(s: &Session, ms: u64) -> bool {
    for _ in 0..ms.div_ceil(40) {
        if s.action.load(Ordering::Acquire) != 0 {
            return false;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    true
}
fn frame(s: &Session) -> Result<RawFrame, CaptureError> {
    native::visible(&s.target, &s.display, s.rect)?;
    if !capture::enumerate_displays()?
        .iter()
        .any(|d| d == &s.display)
    {
        return Err(failure("显示器配置变化，请重新选择。"));
    }
    crop_frame(capture::capture_display(&s.display.id)?, s.rect)
}
fn stable(s: &Session) -> Result<RawFrame, CaptureError> {
    let mut previous = frame(s)?;
    for _ in 0..12 {
        if !sleep(s, 160) {
            return Err(CaptureError::new(ErrorCode::Cancelled, "已中止当前采集。"));
        }
        let next = frame(s)?;
        if frame_difference(&previous, &next) < 0.5 {
            return Ok(next);
        }
        previous = next;
    }
    Err(failure(
        "页面仍在动画或动态加载，暂不拼接。请等待稳定，或缩小选区后重试。",
    ))
}
#[tauri::command]
pub async fn scroll_start(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, Arc<ScrollService>>,
    session_id: String,
) -> Result<(), CaptureError> {
    main(&window)?;
    let s = service.session(&session_id, "main")?;
    if s.action.load(Ordering::Acquire) != 0 || s.started.swap(true, Ordering::AcqRel) {
        return Err(failure("会话已开始或结束，请重新选择。"));
    }
    let label = format!("scroll-control-{}", s.id);
    let built = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App(format!("index.html?scrollControl={}", s.id).into()),
    )
    .title("ResearchClip 长截图控制")
    .inner_size(440.0, 170.0)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(false)
    .build();
    let controller = match built {
        Ok(w) => w,
        Err(_) => {
            s.update("failed", "无法创建长截图控制窗。");
            s.release();
            return Err(failure("无法创建长截图控制窗。"));
        }
    };
    let _ = controller.set_position(PhysicalPosition::new(
        s.display.x_px + 12,
        s.display.y_px + 12,
    ));
    let events = s.clone();
    controller.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            api.prevent_close();
            events.action.store(2, Ordering::Release);
        }
    });
    let task_s = s.clone();
    let task_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let work_s = task_s.clone();
        let work_app = task_app.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            run(&work_app, &window, &controller, &work_s)
        })
        .await;
        if result.is_err() {
            task_s.update("failed", "长截图任务异常终止，临时图像已释放。请重新开始。");
        }
        task_s.release();
        if let Some(w) = task_app.get_webview_window(&format!("scroll-control-{}", task_s.id))
            && w.destroy().is_err()
        {
            let _ = w.hide();
        }
        if let Some(w) = task_app.get_webview_window("main")
            && w.show().and_then(|_| w.set_focus()).is_err()
        {
            eprintln!("ResearchClip: scroll window restoration failed");
        }
    });
    Ok(())
}
fn run(app: &AppHandle, window: &WebviewWindow, controller: &WebviewWindow, s: &Session) {
    let result = (|| -> Result<(), CaptureError> {
        window.hide().map_err(|_| failure("无法隐藏主窗口。"))?;
        native::activate(&s.target)?;
        if !sleep(s, 400) {
            return Err(CaptureError::new(ErrorCode::Cancelled, "已取消长截图。"));
        }
        native::cursor(&s.target, &s.display, s.rect)?;
        let mut engine = StitchingEngine::new(stable(s)?)?;
        s.update(
            "capturing",
            "第一帧已采集。手动模式请向下滚动约半屏，再点击采集下一段。",
        );
        {
            let mut p = s
                .progress
                .lock()
                .map_err(|_| failure("长截图状态不可用。"))?;
            p.frames = 1;
        }
        controller
            .show()
            .map_err(|_| failure("无法显示长截图控制窗。"))?;
        let start = Instant::now();
        let mut auto = s.status()?.automatic;
        loop {
            let action = s.action.load(Ordering::Acquire);
            if action == 2 {
                return Err(CaptureError::new(
                    ErrorCode::Cancelled,
                    "已取消长截图，临时图像已释放。",
                ));
            }
            if start.elapsed() > Duration::from_secs(180) {
                s.update(
                    "capturing",
                    "达到180秒时限，仅保留已采集部分；如需更多内容请分段继续。",
                );
                break;
            }
            if action == 1 {
                break;
            }
            if !auto && !s.next.swap(false, Ordering::AcqRel) {
                std::thread::sleep(Duration::from_millis(40));
                continue;
            }
            if auto && !sleep(s, 800) {
                continue;
            }
            controller
                .hide()
                .map_err(|_| failure("无法暂时隐藏控制窗。"))?;
            let step = (|| {
                if !auto {
                    native::activate(&s.target)?;
                }
                if !sleep(s, 200) {
                    return Err(CaptureError::new(ErrorCode::Cancelled, "采集已中止。"));
                }
                if auto {
                    native::wheel(&s.target, &s.display, s.rect)?;
                    if !sleep(s, 600) {
                        return Err(CaptureError::new(ErrorCode::Cancelled, "滚动已中止。"));
                    }
                }
                engine.append(stable(s)?)
            })();
            controller.show().map_err(|_| failure("无法恢复控制窗。"))?;
            if s.action.load(Ordering::Acquire) != 0 {
                continue;
            }
            match step {
                Ok(Append::Added(offset)) => {
                    s.update("capturing",&format!("已追加{offset}像素。请勿移动目标窗口；固定顶栏仅保留首屏，悬浮底栏请排除。"));
                }
                Ok(Append::Repeated) => {
                    s.update("capturing", "画面未变化；可能已到底部或滚动未生效。");
                    if auto && engine.repeats >= 2 {
                        s.update(
                            "capturing",
                            "连续两次画面未变化，按疑似底部停止；请检查最终图片是否完整。",
                        );
                        break;
                    }
                }
                Err(e) => {
                    auto = false;
                    s.update(
                        "paused",
                        &format!("{} 可手动滚动后重试，或停止保留已拼接部分。", e.message),
                    );
                }
            }
            let mut p = s
                .progress
                .lock()
                .map_err(|_| failure("长截图状态不可用。"))?;
            p.automatic = auto;
            p.frames = engine.frames;
            p.height = engine.height;
        }
        if s.action.load(Ordering::Acquire) == 2 {
            return Err(CaptureError::new(ErrorCode::Cancelled, "已取消。"));
        }
        let reason = s.status()?.message;
        s.update("encoding", "正在生成最终PNG，请稍候…");
        let png = encode_png(engine.finish())?;
        if s.action.load(Ordering::Acquire) == 2 {
            return Err(CaptureError::new(ErrorCode::Cancelled, "已取消。"));
        }
        let info =
            app.state::<Arc<CaptureWorkflow>>()
                .store(s.display.id.clone(), "scroll", png)?;
        {
            let mut p = s
                .progress
                .lock()
                .map_err(|_| failure("长截图状态不可用。"))?;
            p.width = info.width_px;
            p.height = info.height_px;
            p.result = Some(info.clone());
            p.state = "done".into();
            p.message = format!("长图已生成；请核对接缝和完整性后保存。{reason}");
        }
        app.emit_to(
            "main",
            "capture-workflow",
            serde_json::json!({"kind":"ready","capture":info}),
        )
        .map_err(|_| failure("长图已生成，但主窗口通知失败；重新打开主窗口可恢复当前截图。"))?;
        Ok(())
    })();
    if let Err(e) = result {
        s.update(
            if e.code == ErrorCode::Cancelled {
                "cancelled"
            } else {
                "failed"
            },
            &e.message,
        );
    }
}
