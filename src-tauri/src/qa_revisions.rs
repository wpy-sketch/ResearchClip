// Isolated, explicit QA workspace only. No desktop input or production AppData.
use crate::{capture::RawFrame, services::screen_capture::encode_png, storage::Storage};
pub fn run(root: std::path::PathBuf, phase: &str) {
    let mut storage = Storage::open(root.clone()).unwrap();
    let document = |text: &str| serde_json::json!({"schemaVersion":1,"image":{"widthPx":32,"heightPx":24},"objects":[{"id":"text","type":"text","x":2,"y":3,"text":text,"fontSize":12,"color":"#123456","strokeWidth":2}]});
    let png = encode_png(RawFrame {
        width_px: 32,
        height_px: 24,
        rgba: [1, 2, 3, 128].repeat(32 * 24),
    })
    .unwrap();
    if phase == "seed" {
        assert_eq!(storage.list_screenshots(0, 50, false).unwrap().len(), 0);
        let first = storage
            .create_screenshot("历史进程测试", document("初始"), &png)
            .unwrap();
        let old = storage.annotation_history(&first.id, i64::MAX, 50).unwrap()[0]
            .revision_id
            .clone();
        let second = storage
            .save_annotation_revision(&first.id, first.revision, document("第二版"), "修改")
            .unwrap();
        let third = storage
            .restore_annotation_revision(&first.id, second.revision, &old, "恢复初始")
            .unwrap();
        assert_eq!(third.annotation_version, 3);
        std::fs::write(root.join("qa-id.txt"), first.id).unwrap();
    } else {
        assert_eq!(phase, "verify");
        let id = std::fs::read_to_string(root.join("qa-id.txt")).unwrap();
        let current = storage.get_screenshot(&id).unwrap();
        assert_eq!(current.annotation_version, 3);
        assert_eq!(current.annotation_data, document("初始"));
        assert_eq!(
            storage.annotation_history(&id, i64::MAX, 50).unwrap().len(),
            3
        );
        assert_eq!(storage.screenshot_image(&id, false).unwrap(), png);
    }
    println!(
        "{{\"result\":\"PASS\",\"phase\":\"{}\",\"schemaVersion\":5}}",
        phase
    );
}
