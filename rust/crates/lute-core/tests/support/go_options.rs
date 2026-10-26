// Option delta adapters for original Go test drivers. Go field names are preserved.
use lute_core::lute::Lute;
use serde_json::Value;
pub fn apply_parse_options(engine: &mut Lute, options: &Value) {
    let Some(options) = options.as_object() else {
        return;
    };
    for (key, value) in options {
        match key.as_str() {
            "AliasEmoji" => {
                engine.parse_options.alias_emoji =
                    serde_json::from_value(value.clone()).expect("Go parse_options.AliasEmoji")
            }
            "ArbitraryTaskListItemMarker" => {
                engine.parse_options.arbitrary_task_list_item_marker =
                    serde_json::from_value(value.clone())
                        .expect("Go parse_options.ArbitraryTaskListItemMarker")
            }
            "BlockRef" => {
                engine.parse_options.block_ref =
                    serde_json::from_value(value.clone()).expect("Go parse_options.BlockRef")
            }
            "Callout" => {
                engine.parse_options.callout =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Callout")
            }
            "CustomBlock" => {
                engine.parse_options.custom_block =
                    serde_json::from_value(value.clone()).expect("Go parse_options.CustomBlock")
            }
            "DataImage" => {
                engine.parse_options.data_image =
                    serde_json::from_value(value.clone()).expect("Go parse_options.DataImage")
            }
            "Emoji" => {
                engine.parse_options.emoji =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Emoji")
            }
            "EmojiAlias" => {
                engine.parse_options.emoji_alias =
                    serde_json::from_value(value.clone()).expect("Go parse_options.EmojiAlias")
            }
            "EmojiSite" => {
                engine.parse_options.emoji_site =
                    serde_json::from_value(value.clone()).expect("Go parse_options.EmojiSite")
            }
            "EnsureListItemParagraph" => {
                engine.parse_options.ensure_list_item_paragraph =
                    serde_json::from_value(value.clone())
                        .expect("Go parse_options.EnsureListItemParagraph")
            }
            "FileAnnotationRef" => {
                engine.parse_options.file_annotation_ref = serde_json::from_value(value.clone())
                    .expect("Go parse_options.FileAnnotationRef")
            }
            "GFMAutoLink" => {
                engine.parse_options.gfm_auto_link =
                    serde_json::from_value(value.clone()).expect("Go parse_options.GFMAutoLink")
            }
            "GFMStrikethrough" => {
                engine.parse_options.gfm_strikethrough = serde_json::from_value(value.clone())
                    .expect("Go parse_options.GFMStrikethrough")
            }
            "GFMStrikethrough1" => {
                engine.parse_options.gfm_strikethrough1 = serde_json::from_value(value.clone())
                    .expect("Go parse_options.GFMStrikethrough1")
            }
            "GFMTable" => {
                engine.parse_options.gfm_table =
                    serde_json::from_value(value.clone()).expect("Go parse_options.GFMTable")
            }
            "GFMTaskListItem" => {
                engine.parse_options.gfm_task_list_item =
                    serde_json::from_value(value.clone()).expect("Go parse_options.GFMTaskListItem")
            }
            "GitConflict" => {
                engine.parse_options.git_conflict =
                    serde_json::from_value(value.clone()).expect("Go parse_options.GitConflict")
            }
            "HTML2MarkdownAttrs" => {
                engine.parse_options.html2_markdown_attrs = serde_json::from_value(value.clone())
                    .expect("Go parse_options.HTML2MarkdownAttrs")
            }
            "HTMLTag2TextMark" => {
                engine.parse_options.html_tag2_text_mark = serde_json::from_value(value.clone())
                    .expect("Go parse_options.HTMLTag2TextMark")
            }
            "HeadingID" => {
                engine.parse_options.heading_id =
                    serde_json::from_value(value.clone()).expect("Go parse_options.HeadingID")
            }
            "ImgPathAllowSpace" => {
                engine.parse_options.img_path_allow_space = serde_json::from_value(value.clone())
                    .expect("Go parse_options.ImgPathAllowSpace")
            }
            "IndentCodeBlock" => {
                engine.parse_options.indent_code_block =
                    serde_json::from_value(value.clone()).expect("Go parse_options.IndentCodeBlock")
            }
            "InlineAsterisk" => {
                engine.parse_options.inline_asterisk =
                    serde_json::from_value(value.clone()).expect("Go parse_options.InlineAsterisk")
            }
            "InlineMath" => {
                engine.parse_options.inline_math =
                    serde_json::from_value(value.clone()).expect("Go parse_options.InlineMath")
            }
            "InlineMathAllowDigitAfterOpenMarker" => {
                engine
                    .parse_options
                    .inline_math_allow_digit_after_open_marker =
                    serde_json::from_value(value.clone())
                        .expect("Go parse_options.InlineMathAllowDigitAfterOpenMarker")
            }
            "InlineUnderscore" => {
                engine.parse_options.inline_underscore = serde_json::from_value(value.clone())
                    .expect("Go parse_options.InlineUnderscore")
            }
            "KramdownBlockIAL" => {
                engine.parse_options.kramdown_block_ial = serde_json::from_value(value.clone())
                    .expect("Go parse_options.KramdownBlockIAL")
            }
            "KramdownSpanIAL" => {
                engine.parse_options.kramdown_span_ial =
                    serde_json::from_value(value.clone()).expect("Go parse_options.KramdownSpanIAL")
            }
            "LinkRef" => {
                engine.parse_options.link_ref =
                    serde_json::from_value(value.clone()).expect("Go parse_options.LinkRef")
            }
            "Mark" => {
                engine.parse_options.mark =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Mark")
            }
            "ParagraphBeginningSpace" => {
                engine.parse_options.paragraph_beginning_space =
                    serde_json::from_value(value.clone())
                        .expect("Go parse_options.ParagraphBeginningSpace")
            }
            "ProtyleWYSIWYG" => {
                engine.parse_options.protyle_wysiwyg =
                    serde_json::from_value(value.clone()).expect("Go parse_options.ProtyleWYSIWYG")
            }
            "Setext" => {
                engine.parse_options.setext =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Setext")
            }
            "Spin" => {
                engine.parse_options.spin =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Spin")
            }
            "Sub" => {
                engine.parse_options.sub =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Sub")
            }
            "Sup" => {
                engine.parse_options.sup =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Sup")
            }
            "SuperBlock" => {
                engine.parse_options.super_block =
                    serde_json::from_value(value.clone()).expect("Go parse_options.SuperBlock")
            }
            "Tag" => {
                engine.parse_options.tag =
                    serde_json::from_value(value.clone()).expect("Go parse_options.Tag")
            }
            "TextMark" => {
                engine.parse_options.text_mark =
                    serde_json::from_value(value.clone()).expect("Go parse_options.TextMark")
            }
            "ToC" => {
                engine.parse_options.toc =
                    serde_json::from_value(value.clone()).expect("Go parse_options.ToC")
            }
            "VditorIR" => {
                engine.parse_options.vditor_ir =
                    serde_json::from_value(value.clone()).expect("Go parse_options.VditorIR")
            }
            "VditorSV" => {
                engine.parse_options.vditor_sv =
                    serde_json::from_value(value.clone()).expect("Go parse_options.VditorSV")
            }
            "VditorWYSIWYG" => {
                engine.parse_options.vditor_wysiwyg =
                    serde_json::from_value(value.clone()).expect("Go parse_options.VditorWYSIWYG")
            }
            key => panic!("unmapped Go option: {key}"),
        }
    }
}
pub fn apply_render_options(engine: &mut Lute, options: &Value) {
    let Some(options) = options.as_object() else {
        return;
    };
    for (key, value) in options {
        match key.as_str() {
            "AutoSpace" => {
                engine.render_options.auto_space =
                    serde_json::from_value(value.clone()).expect("Go render_options.AutoSpace")
            }
            "ChineseParagraphBeginningSpace" => {
                engine.render_options.chinese_paragraph_beginning_space =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.ChineseParagraphBeginningSpace")
            }
            "CodeSyntaxHighlight" => {
                engine.render_options.code_syntax_highlight = serde_json::from_value(value.clone())
                    .expect("Go render_options.CodeSyntaxHighlight")
            }
            "CodeSyntaxHighlightDetectLang" => {
                engine.render_options.code_syntax_highlight_detect_lang =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.CodeSyntaxHighlightDetectLang")
            }
            "CodeSyntaxHighlightInlineStyle" => {
                engine.render_options.code_syntax_highlight_inline_style =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.CodeSyntaxHighlightInlineStyle")
            }
            "CodeSyntaxHighlightLineNum" => {
                engine.render_options.code_syntax_highlight_line_num =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.CodeSyntaxHighlightLineNum")
            }
            "CodeSyntaxHighlightStyleName" => {
                engine.render_options.code_syntax_highlight_style_name =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.CodeSyntaxHighlightStyleName")
            }
            "DataTask" => {
                engine.render_options.data_task =
                    serde_json::from_value(value.clone()).expect("Go render_options.DataTask")
            }
            "FixTermTypo" => {
                engine.render_options.fix_term_typo =
                    serde_json::from_value(value.clone()).expect("Go render_options.FixTermTypo")
            }
            "GFMTaskListItemClass" => {
                engine.render_options.gfm_task_list_item_class =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.GFMTaskListItemClass")
            }
            "HeadingAnchor" => {
                engine.render_options.heading_anchor =
                    serde_json::from_value(value.clone()).expect("Go render_options.HeadingAnchor")
            }
            "HeadingID" => {
                engine.render_options.heading_id =
                    serde_json::from_value(value.clone()).expect("Go render_options.HeadingID")
            }
            "ImageLazyLoading" => {
                engine.render_options.image_lazy_loading = serde_json::from_value(value.clone())
                    .expect("Go render_options.ImageLazyLoading")
            }
            "KeepParagraphBeginningSpace" => {
                engine.render_options.keep_paragraph_beginning_space =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.KeepParagraphBeginningSpace")
            }
            "KramdownBlockIAL" => {
                engine.render_options.kramdown_block_ial = serde_json::from_value(value.clone())
                    .expect("Go render_options.KramdownBlockIAL")
            }
            "KramdownIALIDRenderName" => {
                engine.render_options.kramdown_ial_id_render_name =
                    serde_json::from_value(value.clone())
                        .expect("Go render_options.KramdownIALIDRenderName")
            }
            "KramdownSpanIAL" => {
                engine.render_options.kramdown_span_ial = serde_json::from_value(value.clone())
                    .expect("Go render_options.KramdownSpanIAL")
            }
            "LinkBase" => {
                engine.render_options.link_base =
                    serde_json::from_value(value.clone()).expect("Go render_options.LinkBase")
            }
            "LinkPrefix" => {
                engine.render_options.link_prefix =
                    serde_json::from_value(value.clone()).expect("Go render_options.LinkPrefix")
            }
            "ProtyleWYSIWYG" => {
                engine.render_options.protyle_wysiwyg =
                    serde_json::from_value(value.clone()).expect("Go render_options.ProtyleWYSIWYG")
            }
            "RenderListStyle" => {
                engine.render_options.render_list_style = serde_json::from_value(value.clone())
                    .expect("Go render_options.RenderListStyle")
            }
            "Sanitize" => {
                engine.render_options.sanitize =
                    serde_json::from_value(value.clone()).expect("Go render_options.Sanitize")
            }
            "SoftBreak2HardBreak" => {
                engine.render_options.soft_break2_hard_break = serde_json::from_value(value.clone())
                    .expect("Go render_options.SoftBreak2HardBreak")
            }
            "SuperBlock" => {
                engine.render_options.super_block =
                    serde_json::from_value(value.clone()).expect("Go render_options.SuperBlock")
            }
            "Terms" => {
                engine.render_options.terms =
                    serde_json::from_value(value.clone()).expect("Go render_options.Terms")
            }
            "ToC" => {
                engine.render_options.toc =
                    serde_json::from_value(value.clone()).expect("Go render_options.ToC")
            }
            "UnorderedListMarker" => {
                engine.render_options.unordered_list_marker = serde_json::from_value(value.clone())
                    .expect("Go render_options.UnorderedListMarker")
            }
            "VditorIR" => {
                engine.render_options.vditor_ir =
                    serde_json::from_value(value.clone()).expect("Go render_options.VditorIR")
            }
            "VditorSV" => {
                engine.render_options.vditor_sv =
                    serde_json::from_value(value.clone()).expect("Go render_options.VditorSV")
            }
            "VditorWYSIWYG" => {
                engine.render_options.vditor_wysiwyg =
                    serde_json::from_value(value.clone()).expect("Go render_options.VditorWYSIWYG")
            }
            key => panic!("unmapped Go option: {key}"),
        }
    }
}
