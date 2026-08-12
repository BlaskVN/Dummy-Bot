pub mod activity;
pub mod configuration;
pub mod donation;
pub mod general;
pub mod moderation;
pub mod presence;
pub mod reload_modules;
pub mod voice;
pub mod word_puzzle;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    let mut commands = general::all();
    commands.push(activity::activity());
    commands.extend(moderation::all());
    commands.extend(configuration::all());
    commands.extend(voice::all());
    commands.push(donation::donation());
    commands.push(presence::presence());
    commands.push(word_puzzle::word_puzzle());
    commands.push(reload_modules::reload_modules());
    apply_localizations(&mut commands);
    commands
}

fn apply_localizations(commands: &mut [poise::Command<Data, Error>]) {
    for cmd in commands {
        apply_single_command_localization(cmd);
    }
}

fn apply_single_command_localization(cmd: &mut poise::Command<Data, Error>) {
    let (vi_desc, ja_desc) = get_command_descriptions(&cmd.name);
    cmd.description_localizations
        .insert("vi".to_string(), vi_desc);
    cmd.description_localizations
        .insert("ja".to_string(), ja_desc);

    for param in &mut cmd.parameters {
        let (vi_desc, ja_desc) = get_param_descriptions(&cmd.name, &param.name);
        param
            .description_localizations
            .insert("vi".to_string(), vi_desc);
        param
            .description_localizations
            .insert("ja".to_string(), ja_desc);
    }

    for sub in &mut cmd.subcommands {
        apply_single_command_localization(sub);
    }
}

fn get_command_descriptions(name: &str) -> (String, String) {
    let known = match name {
        "ping" => Some((
            "Kiểm tra độ trễ và khả năng phản hồi của Bot.",
            "ボットの応答速度（レイテンシ）を確認します。",
        )),
        "reload_modules" => Some((
            "Nạp lại toàn bộ Rhai script modules mà không cần khởi động lại bot.",
            "ボットを再起動せずにすべての Rhai スクリプトモジュールを再読み込みします。",
        )),
        "botinfo" => Some((
            "Hiển thị thông tin chi tiết và thời gian hoạt động của Bot.",
            "ボットの基本情報と稼働時間を表示します。",
        )),
        "serverinfo" => Some((
            "Hiển thị thông tin chi tiết về Server hiện tại.",
            "現在のサーバー情報を表示します。",
        )),
        "donate" => Some((
            "Hiển thị thông tin ủng hộ/donate đã cấu hình.",
            "設定された寄付情報を表示します。",
        )),
        "ban" => Some((
            "Cấm (Ban) một thành viên và ghi nhận hồ sơ xử phạt.",
            "メンバーを BAN し、処罰記録を作成します。",
        )),
        "kick" => Some((
            "Đuổi (Kick) một thành viên và ghi nhận hồ sơ xử phạt.",
            "メンバーをキックし、処罰記録を作成します。",
        )),
        "timeout" => Some((
            "Tạm thời cấm ngôn (Timeout) thành viên và ghi nhận hồ sơ xử phạt.",
            "メンバーをタイムアウトし、処罰記録を作成します。",
        )),
        "purge" => Some((
            "Xóa hàng loạt tin nhắn gần đây trong kênh này.",
            "このチャンネルの最近のメッセージを一括削除します。",
        )),
        "warn" => Some((
            "Cảnh báo một thành viên và ghi nhận hồ sơ xử phạt.",
            "メンバーに警告を与え、処罰記録を作成します。",
        )),
        "case" => Some((
            "Xem hoặc quản lý các hồ sơ xử phạt (Moderation cases).",
            "処罰ケースの確認・管理を行います。",
        )),
        "language" => Some((
            "Thiết lập ngôn ngữ phản hồi của Bot cho Server này.",
            "このサーバーでのボットの応答言語を設定します。",
        )),
        "timezone" => Some((
            "Cấu hình múi giờ (Timezone) sử dụng cho Server này.",
            "このサーバーで使用するタイムゾーンを設定します。",
        )),
        "messagelog" => Some((
            "Cấu hình log tin nhắn bị xóa và tin nhắn được chỉnh sửa.",
            "メッセージの編集・削除ログを設定します。",
        )),
        "moderation-channel" => Some((
            "Cấu hình kênh riêng tư lưu trữ các hồ sơ xử phạt.",
            "モデレーション記録用の専用チャンネルを設定します。",
        )),
        "game-config" => Some((
            "Cấu hình Role, kênh chat và nhận diện Session voice cho Game.",
            "ゲーム用のロール、チャンネル、ボイスセッション検出を設定します。",
        )),
        "presence" => Some((
            "Cấu hình trạng thái hiển thị (Status/Rich Presence) cho Bot.",
            "ボットのカスタムステータスや Rich Presence を設定します。",
        )),
        "activity" => Some((
            "Xem hoặc quản lý hệ thống theo dõi hoạt động cộng đồng.",
            "コミュニティアクティビティの追跡状況を確認・管理します。",
        )),
        "word-puzzle" => Some((
            "Chơi trò chơi đoán từ tiếng Anh 5 chữ cái cùng đồng đội.",
            "協力型 5 文字英単語パズルゲームをプレイします。",
        )),
        "connect" => Some((
            "Kết nối vào kênh Voice để theo dõi hoạt động.",
            "アクティビティ追跡のためボイスチャンネルに接続します。",
        )),
        "disconnect" => Some((
            "Ngắt kết nối khỏi kênh Voice.",
            "ボイスチャンネルから切断します。",
        )),
        "set" => Some((
            "Cấu hình hoặc thiết lập giá trị mới.",
            "新しい設定値を適用します。",
        )),
        "show" => Some(("Hiển thị cấu hình hiện tại.", "現在の設定を表示します。")),
        "clear" => Some((
            "Xóa hoặc đặt lại cấu hình về mặc định.",
            "設定をクリアまたは初期化します。",
        )),
        "enable" => Some(("Bật tính năng này.", "この機能を有効にします。")),
        "disable" => Some(("Tắt tính năng này.", "この機能を無効にします。")),
        "status" => Some((
            "Xem trạng thái hiện tại của tính năng.",
            "現在の機能ステータスを確認します。",
        )),
        "view" => Some(("Xem chi tiết một hồ sơ.", "詳細情報を確認します。")),
        "void" => Some((
            "Hủy bỏ hoặc vô hiệu hóa một hồ sơ xử phạt.",
            "処罰ケースを無効化します。",
        )),
        "create" => Some((
            "Tạo một phòng/phiên làm việc mới.",
            "新しいセッションを作成します。",
        )),
        "join" => Some((
            "Tham gia vào phiên hiện tại.",
            "現在のセッションに参加します。",
        )),
        "start" => Some(("Bắt đầu trò chơi.", "ゲームを開始します。")),
        "guess" => Some((
            "Đoán từ tiếng Anh 5 chữ cái.",
            "5文字の英単語を推測します。",
        )),
        "finish" => Some((
            "Kết thúc và tổng kết trò chơi.",
            "ゲームを終了して結果を表示します。",
        )),
        "cancel" => Some((
            "Hủy bỏ hoạt động hoặc sự kiện đang diễn ra.",
            "進行中のアクティビティやイベントをキャンセルします。",
        )),
        "update" => Some((
            "Cập nhật hoặc chỉnh sửa thiết lập.",
            "設定の更新または編集を行います。",
        )),
        "opt-in" => Some((
            "Đăng ký tham gia theo dõi dữ liệu.",
            "データ追跡への参加を登録します。",
        )),
        "opt-out" => Some((
            "Hủy đăng ký và xóa dữ liệu theo dõi.",
            "データ追跡の解除とデータの削除を行います。",
        )),
        "admin" => Some((
            "Quản trị và cấu hình nâng cao.",
            "管理者向けの高度な設定を行います。",
        )),
        "report" => Some((
            "Xuất báo cáo thống kê chi tiết.",
            "詳細な統計レポートを出力します。",
        )),
        "setup" => Some((
            "Thiết lập và cấu hình ban đầu.",
            "初期セットアップを行います。",
        )),
        "info" => Some(("Xem thông tin chi tiết.", "詳細情報を表示します。")),
        _ => None,
    };

    if let Some((vi, ja)) = known {
        (vi.to_string(), ja.to_string())
    } else {
        (
            format!("Thực hiện lệnh {name}."),
            format!("コマンド {name} を実行します。"),
        )
    }
}

fn get_param_descriptions(cmd_name: &str, param_name: &str) -> (String, String) {
    let known = match (cmd_name, param_name) {
        (_, "member") => Some((
            "Thành viên được chọn để thực hiện thao tác.",
            "対象のサーバーメンバー。",
        )),
        (_, "reason") => Some(("Lý do thực hiện thao tác này.", "この操作を実行する理由。")),
        (_, "evidence") => Some((
            "Đường dẫn liên kết tin nhắn Discord chứa bằng chứng.",
            "証拠となる Discord メッセージのリンク。",
        )),
        ("ban", "delete_days") => Some((
            "Số ngày tin nhắn gần đây cần xóa (0-7 ngày).",
            "削除する過去メッセージの日数 (0〜7日)。",
        )),
        ("timeout", "minutes") => Some((
            "Thời gian cấm ngôn tính theo phút.",
            "タイムアウトの長さ（分単位）。",
        )),
        ("purge", "amount") => Some((
            "Số lượng tin nhắn muốn xóa (từ 1 đến 100).",
            "削除するメッセージ数（1〜100）。",
        )),
        ("language", "lang_code") => {
            Some(("Mã ngôn ngữ (en, vi, ja).", "言語コード (en, vi, ja)。"))
        }
        ("timezone", "iana_name") => Some((
            "Tên múi giờ IANA chuẩn, ví dụ Asia/Ho_Chi_Minh.",
            "標準 IANA タイムゾーン名（例：Asia/Tokyo）。",
        )),
        ("case", "case_number") => Some(("Mã số hồ sơ xử phạt.", "処罰ケース番号。")),
        ("word-puzzle", "word") => Some((
            "Từ tiếng Anh 5 chữ cái bạn muốn đoán.",
            "推測する 5文字の英単語。",
        )),
        _ => None,
    };

    if let Some((vi, ja)) = known {
        (vi.to_string(), ja.to_string())
    } else {
        (
            format!("Tham số {param_name} cho lệnh."),
            format!("コマンドのパラメータ {param_name}。"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::all;

    #[test]
    fn registers_representative_slash_commands_without_prefix_actions() {
        fn inspect(command: &poise::Command<crate::Data, crate::Error>) {
            let description = command.description.as_deref().unwrap_or_default();
            assert!(
                !description.is_empty() && description != "A slash command",
                "/{} has a missing or default description",
                command.name
            );
            assert!(
                command.slash_action.is_some(),
                "{} is not slash-enabled",
                command.name
            );
            assert!(
                command.description_localizations.contains_key("vi")
                    && command.description_localizations.contains_key("ja"),
                "/{} is missing description localizations",
                command.name
            );
            assert!(
                command.prefix_action.is_none(),
                "{} still has prefix dispatch",
                command.name
            );
            for parameter in &command.parameters {
                let description = parameter.description.as_deref().unwrap_or_default();
                assert!(
                    !description.is_empty() && description != "A slash command parameter",
                    "/{} parameter {} has a missing or default description",
                    command.name,
                    parameter.name
                );
            }
            for subcommand in &command.subcommands {
                inspect(subcommand);
            }
        }

        let commands = all();
        for command in &commands {
            inspect(command);
        }
        let names = commands
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>();
        for representative in [
            "ping",
            "ban",
            "settings",
            "presence",
            "connect",
            "reload_modules",
        ] {
            assert!(names.contains(&representative), "missing /{representative}");
        }
        assert!(!names.contains(&"setprefix"));
        assert!(!names.contains(&"valorant"));
    }
}
