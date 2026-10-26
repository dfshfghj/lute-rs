/* tslint:disable */
/* eslint-disable */

export function BlockDOM2ContentGlobal(dom: string): string;

export function EChartsMindmapStr(s: string): string;

export function EscapeHTMLStr(s: string): string;

export function GetHeadingID(node: JsNode): string;

export function GetHeadingIDRaw(node: JsNode): string;

export class JsNode {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    AppendChild(child: JsNode): void;
    ChildByType(typ: string): any;
    ChildrenByType(typ: string): Array<any>;
    Content(): string;
    FirstDeepestChild(): JsNode;
    IALAttr(name: string): string;
    InsertAfter(sibling: JsNode): void;
    InsertBefore(sibling: JsNode): void;
    IsBlock(): boolean;
    LastDeepestChild(): JsNode;
    NextNodeText(): string;
    ParentIs(typ: string): boolean;
    PreviousNodeText(): string;
    SetIALAttr(name: string, value: string): void;
    TextLen(): number;
    Text(): string;
    TokensStr(): string;
    Unlink(): void;
    /**
     * 对应 Go 版包装对象的 __internal_object__：字段访问走这里。
     */
    readonly __internal_object__: any;
}

export class JsTree {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    Root(): JsNode;
    readonly ID: string;
    readonly Name: string;
}

/**
 * 对应 Go 版 lute.Lute 的 JS 包装。
 */
export class LuteEngine {
    free(): void;
    [Symbol.dispose](): void;
    BlockDOM2Content(blockdom: string): string;
    BlockDOM2EscapeMarkerContent(blockdom: string): string;
    BlockDOM2HTML(blockdom: string): string;
    BlockDOM2InlineBlockDOM(blockdom: string): string;
    BlockDOM2Md(blockdom: string): string;
    BlockDOM2RichHTML(blockdom: string): string;
    BlockDOM2StdMd(blockdom: string): string;
    BlockDOM2TextLen(blockdom: string): number;
    BlockDOM2Text(blockdom: string): string;
    BlockDOM2Tree(blockdom: string): JsTree;
    Blockquote2Callout(blockdom: string): string;
    Blocks2Hs(blockdom: string, level: string): string;
    Blocks2Ps(blockdom: string): string;
    Callout2Blockquote(blockdom: string): string;
    CancelBlockquote(blockdom: string): string;
    CancelCallout(blockdom: string): string;
    CancelListRecursively(blockdom: string): string;
    CancelList(blockdom: string): string;
    CancelSuperBlock(blockdom: string): string;
    ConvertListType(blockdom: string, target: string): string;
    FormatStr(name: string, markdown: string): string;
    Format(name: string, markdown: Uint8Array): Uint8Array;
    GetEmojis(): object;
    GetLinkBase(): string;
    GetLinkDest(s: string): string;
    GetTerms(): object;
    HTML2BlockDOM(html: string): string;
    HTML2Markdown(dom: string): Array<any>;
    HTML2Md(dom: string): string;
    HTML2Text(dom: string): string;
    HTML2Tree(html: string): JsTree;
    HTML2VditorDOM(html: string): string;
    HTML2VditorIRDOM(html: string): string;
    HTML2VditorSVDOM(html: string): string;
    /**
     * 对应 Go 版 Lute.HTMLNode2Tree。JS 侧没有 html.Node 包装，这里接受 HTML 字符串。
     */
    HTMLNode2Tree(node: string): JsTree;
    InlineMd2BlockDOM(markdown: string): string;
    IsValidLinkDest(s: string): boolean;
    MarkdownStr(name: string, markdown: string): string;
    Markdown(name: string, markdown: Uint8Array): Uint8Array;
    Md2BlockDOMTree(markdown: string, reserve_empty_paragraph: boolean): Array<any>;
    Md2BlockDOMWithAutoLink(markdown: string, reserve_empty_paragraph: boolean): string;
    Md2BlockDOM(markdown: string, reserve_empty_paragraph: boolean): string;
    Md2HTML(markdown: string): string;
    Md2VditorDOM(markdown: string): string;
    Md2VditorIRDOM(markdown: string): string;
    Md2VditorSVDOM(markdown: string): string;
    MergeSameSpan(node: JsNode): void;
    MergeSameTextMark(node: JsNode): void;
    OL2TL(blockdom: string): string;
    OL2UL(blockdom: string): string;
    ProtylePreviewStr(name: string, markdown: string): string;
    ProtylePreview(name: string, markdown: Uint8Array): Uint8Array;
    PutEmojis(emojis: object): void;
    PutTerms(terms: object): void;
    RemoveEmoji(s: string): string;
    RenderEChartsJSON(markdown: string): string;
    RenderJSON(markdown: string): string;
    RenderKityMinderJSON(markdown: string): string;
    RenderNodeBlockDOM(node: JsNode): string;
    SetArbitraryTaskListItemMarker(b: boolean): void;
    SetAutoSpace(b: boolean): void;
    SetBlockRef(b: boolean): void;
    SetCallout(b: boolean): void;
    SetChineseParagraphBeginningSpace(b: boolean): void;
    SetCodeSyntaxHighlightDetectLang(b: boolean): void;
    SetCodeSyntaxHighlightInlineStyle(b: boolean): void;
    SetCodeSyntaxHighlightLineNum(b: boolean): void;
    SetCodeSyntaxHighlightStyleName(name: string): void;
    SetCodeSyntaxHighlight(b: boolean): void;
    SetCustomBlock(b: boolean): void;
    SetDataImage(b: boolean): void;
    SetDataTask(b: boolean): void;
    SetEmojiSite(site: string): void;
    SetEmoji(b: boolean): void;
    SetEmojis(emojis: object): void;
    SetEnsureListItemParagraph(b: boolean): void;
    SetExportNormalizeTaskListMarker(b: boolean): void;
    SetFileAnnotationRef(b: boolean): void;
    SetFixTermTypo(b: boolean): void;
    SetFootnotes(b: boolean): void;
    SetFullWidthStrikethrough(b: boolean): void;
    SetGFMAutoLink(b: boolean): void;
    SetGFMStrikethrough1(b: boolean): void;
    SetGFMStrikethrough(b: boolean): void;
    SetGFMTable(b: boolean): void;
    SetGFMTaskListItemClass(_class: string): void;
    SetGFMTaskListItem(b: boolean): void;
    SetGitConflict(b: boolean): void;
    SetHTML2MarkdownAttrs(attrs: Array<any>): void;
    SetHTMLTag2TextMark(b: boolean): void;
    SetHeadingAnchor(b: boolean): void;
    SetHeadingID(b: boolean): void;
    SetImageLazyLoading(data_src: string): void;
    SetImgPathAllowSpace(b: boolean): void;
    SetImgTag(b: boolean): void;
    SetIndentCodeBlock(b: boolean): void;
    SetInlineAsterisk(b: boolean): void;
    SetInlineMathAllowDigitAfterOpenMarker(b: boolean): void;
    SetInlineMath(b: boolean): void;
    SetInlineUnderscore(b: boolean): void;
    /**
     * 对应 Go 版 Lute.SetJSRenderers。
     */
    SetJSRenderers(options: any): void;
    SetKramdownBlockIAL(b: boolean): void;
    SetKramdownIALIDRenderName(name: string): void;
    SetKramdownIAL(b: boolean): void;
    SetKramdownSpanIAL(b: boolean): void;
    SetLinkBase(base: string): void;
    SetLinkPrefix(prefix: string): void;
    SetLinkRef(b: boolean): void;
    SetMark(b: boolean): void;
    SetParagraphBeginningSpace(b: boolean): void;
    SetPreventEncodeLinkSpace(b: boolean): void;
    SetProtyleMarkNetImg(b: boolean): void;
    SetProtyleWYSIWYG(b: boolean): void;
    SetRenderListStyle(b: boolean): void;
    SetSanitize(b: boolean): void;
    SetSetext(b: boolean): void;
    SetSoftBreak2HardBreak(b: boolean): void;
    SetSpellcheck(b: boolean): void;
    SetSpin(b: boolean): void;
    SetSub(b: boolean): void;
    SetSup(b: boolean): void;
    SetSuperBlock(b: boolean): void;
    SetTabs(b: boolean): void;
    SetTag(b: boolean): void;
    SetTerms(terms: object): void;
    SetTextMark(b: boolean): void;
    SetToC(b: boolean): void;
    SetUnorderedListMarker(marker: string): void;
    SetVditorCodeBlockPreview(b: boolean): void;
    SetVditorHTMLBlockPreview(b: boolean): void;
    SetVditorIR(b: boolean): void;
    SetVditorMathBlockPreview(b: boolean): void;
    SetVditorSV(b: boolean): void;
    SetVditorWYSIWYG(b: boolean): void;
    SetYamlFrontMatter(b: boolean): void;
    Space(text: string): string;
    SpinBlockDOM(blockdom: string): string;
    SpinVditorDOM(vhtml: string): string;
    SpinVditorIRDOM(vhtml: string): string;
    SpinVditorSVDOM(markdown: string): string;
    TL2OL(blockdom: string): string;
    TL2UL(blockdom: string): string;
    TextBundleStr(name: string, markdown: string, link_prefixes: Array<any>): Array<any>;
    TextBundle(name: string, markdown: Uint8Array, link_prefixes: Array<any>): Array<any>;
    Tree2BlockDOM(tree: JsTree, _options: any, _parse_options: any): string;
    Tree2HTML(tree: JsTree, _options: any, _parse_options: any): string;
    UL2OL(blockdom: string): string;
    UL2TL(blockdom: string): string;
    VditorDOM2HTML(vhtml: string): string;
    VditorDOM2Md(vhtml: string): string;
    VditorIRDOM2HTML(vhtml: string): string;
    VditorIRDOM2Md(vhtml: string): string;
    constructor();
}

export function NewNodeID(): string;

export function Sanitize(s: string): string;

export function UnEscapeHTMLStr(s: string): string;

export function luteVersion(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly BlockDOM2ContentGlobal: (a: number, b: number) => [number, number];
    readonly EChartsMindmapStr: (a: number, b: number) => [number, number];
    readonly EscapeHTMLStr: (a: number, b: number) => [number, number];
    readonly GetHeadingID: (a: number) => [number, number];
    readonly GetHeadingIDRaw: (a: number) => [number, number];
    readonly NewNodeID: () => [number, number];
    readonly Sanitize: (a: number, b: number) => [number, number];
    readonly UnEscapeHTMLStr: (a: number, b: number) => [number, number];
    readonly __wbg_jsnode_free: (a: number, b: number) => void;
    readonly __wbg_jstree_free: (a: number, b: number) => void;
    readonly __wbg_luteengine_free: (a: number, b: number) => void;
    readonly jsnode_AppendChild: (a: number, b: number) => void;
    readonly jsnode_ChildByType: (a: number, b: number, c: number) => any;
    readonly jsnode_ChildrenByType: (a: number, b: number, c: number) => any;
    readonly jsnode_Content: (a: number) => [number, number];
    readonly jsnode_FirstDeepestChild: (a: number) => number;
    readonly jsnode_IALAttr: (a: number, b: number, c: number) => [number, number];
    readonly jsnode_InsertAfter: (a: number, b: number) => void;
    readonly jsnode_InsertBefore: (a: number, b: number) => void;
    readonly jsnode_IsBlock: (a: number) => number;
    readonly jsnode_LastDeepestChild: (a: number) => number;
    readonly jsnode_NextNodeText: (a: number) => [number, number];
    readonly jsnode_ParentIs: (a: number, b: number, c: number) => number;
    readonly jsnode_PreviousNodeText: (a: number) => [number, number];
    readonly jsnode_SetIALAttr: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly jsnode_Text: (a: number) => [number, number];
    readonly jsnode_TextLen: (a: number) => number;
    readonly jsnode_TokensStr: (a: number) => [number, number];
    readonly jsnode_Unlink: (a: number) => void;
    readonly jsnode___internal_object__: (a: number) => any;
    readonly jstree_ID: (a: number) => [number, number];
    readonly jstree_Name: (a: number) => [number, number];
    readonly jstree_Root: (a: number) => number;
    readonly luteVersion: () => [number, number];
    readonly luteengine_BlockDOM2Content: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2EscapeMarkerContent: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2HTML: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2InlineBlockDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_BlockDOM2Md: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2RichHTML: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2StdMd: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2Text: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_BlockDOM2TextLen: (a: number, b: number, c: number) => number;
    readonly luteengine_BlockDOM2Tree: (a: number, b: number, c: number) => number;
    readonly luteengine_Blockquote2Callout: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_Blocks2Hs: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly luteengine_Blocks2Ps: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_Callout2Blockquote: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_CancelBlockquote: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_CancelCallout: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_CancelList: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_CancelListRecursively: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_CancelSuperBlock: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_ConvertListType: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly luteengine_Format: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly luteengine_FormatStr: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly luteengine_GetEmojis: (a: number) => any;
    readonly luteengine_GetLinkBase: (a: number) => [number, number];
    readonly luteengine_GetLinkDest: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_GetTerms: (a: number) => any;
    readonly luteengine_HTML2BlockDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_HTML2Markdown: (a: number, b: number, c: number) => [number, number, number];
    readonly luteengine_HTML2Md: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_HTML2Text: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_HTML2Tree: (a: number, b: number, c: number) => number;
    readonly luteengine_HTML2VditorDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_HTML2VditorIRDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_HTML2VditorSVDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_HTMLNode2Tree: (a: number, b: number, c: number) => number;
    readonly luteengine_InlineMd2BlockDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_IsValidLinkDest: (a: number, b: number, c: number) => number;
    readonly luteengine_Markdown: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly luteengine_MarkdownStr: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly luteengine_Md2BlockDOM: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly luteengine_Md2BlockDOMTree: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly luteengine_Md2BlockDOMWithAutoLink: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly luteengine_Md2HTML: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_Md2VditorDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_Md2VditorIRDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_Md2VditorSVDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_MergeSameSpan: (a: number, b: number) => void;
    readonly luteengine_MergeSameTextMark: (a: number, b: number) => void;
    readonly luteengine_OL2TL: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_OL2UL: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_ProtylePreview: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly luteengine_ProtylePreviewStr: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly luteengine_PutEmojis: (a: number, b: any) => void;
    readonly luteengine_PutTerms: (a: number, b: any) => void;
    readonly luteengine_RemoveEmoji: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_RenderEChartsJSON: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_RenderJSON: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_RenderKityMinderJSON: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_RenderNodeBlockDOM: (a: number, b: number) => [number, number, number, number];
    readonly luteengine_SetArbitraryTaskListItemMarker: (a: number, b: number) => void;
    readonly luteengine_SetAutoSpace: (a: number, b: number) => [number, number];
    readonly luteengine_SetBlockRef: (a: number, b: number) => void;
    readonly luteengine_SetCallout: (a: number, b: number) => void;
    readonly luteengine_SetChineseParagraphBeginningSpace: (a: number, b: number) => void;
    readonly luteengine_SetCodeSyntaxHighlight: (a: number, b: number) => void;
    readonly luteengine_SetCodeSyntaxHighlightDetectLang: (a: number, b: number) => void;
    readonly luteengine_SetCodeSyntaxHighlightInlineStyle: (a: number, b: number) => void;
    readonly luteengine_SetCodeSyntaxHighlightLineNum: (a: number, b: number) => void;
    readonly luteengine_SetCodeSyntaxHighlightStyleName: (a: number, b: number, c: number) => void;
    readonly luteengine_SetCustomBlock: (a: number, b: number) => void;
    readonly luteengine_SetDataImage: (a: number, b: number) => void;
    readonly luteengine_SetDataTask: (a: number, b: number) => void;
    readonly luteengine_SetEmoji: (a: number, b: number) => void;
    readonly luteengine_SetEmojiSite: (a: number, b: number, c: number) => void;
    readonly luteengine_SetEmojis: (a: number, b: any) => void;
    readonly luteengine_SetEnsureListItemParagraph: (a: number, b: number) => void;
    readonly luteengine_SetExportNormalizeTaskListMarker: (a: number, b: number) => void;
    readonly luteengine_SetFileAnnotationRef: (a: number, b: number) => void;
    readonly luteengine_SetFixTermTypo: (a: number, b: number) => void;
    readonly luteengine_SetFootnotes: (a: number, b: number) => void;
    readonly luteengine_SetFullWidthStrikethrough: (a: number, b: number) => void;
    readonly luteengine_SetGFMAutoLink: (a: number, b: number) => void;
    readonly luteengine_SetGFMStrikethrough: (a: number, b: number) => void;
    readonly luteengine_SetGFMStrikethrough1: (a: number, b: number) => void;
    readonly luteengine_SetGFMTable: (a: number, b: number) => void;
    readonly luteengine_SetGFMTaskListItem: (a: number, b: number) => void;
    readonly luteengine_SetGFMTaskListItemClass: (a: number, b: number, c: number) => void;
    readonly luteengine_SetGitConflict: (a: number, b: number) => void;
    readonly luteengine_SetHTML2MarkdownAttrs: (a: number, b: any) => void;
    readonly luteengine_SetHTMLTag2TextMark: (a: number, b: number) => void;
    readonly luteengine_SetHeadingAnchor: (a: number, b: number) => void;
    readonly luteengine_SetHeadingID: (a: number, b: number) => void;
    readonly luteengine_SetImageLazyLoading: (a: number, b: number, c: number) => void;
    readonly luteengine_SetImgPathAllowSpace: (a: number, b: number) => void;
    readonly luteengine_SetImgTag: (a: number, b: number) => void;
    readonly luteengine_SetIndentCodeBlock: (a: number, b: number) => void;
    readonly luteengine_SetInlineAsterisk: (a: number, b: number) => void;
    readonly luteengine_SetInlineMath: (a: number, b: number) => void;
    readonly luteengine_SetInlineMathAllowDigitAfterOpenMarker: (a: number, b: number) => void;
    readonly luteengine_SetInlineUnderscore: (a: number, b: number) => void;
    readonly luteengine_SetJSRenderers: (a: number, b: any) => [number, number];
    readonly luteengine_SetKramdownBlockIAL: (a: number, b: number) => [number, number];
    readonly luteengine_SetKramdownIAL: (a: number, b: number) => void;
    readonly luteengine_SetKramdownIALIDRenderName: (a: number, b: number, c: number) => void;
    readonly luteengine_SetKramdownSpanIAL: (a: number, b: number) => void;
    readonly luteengine_SetLinkBase: (a: number, b: number, c: number) => void;
    readonly luteengine_SetLinkPrefix: (a: number, b: number, c: number) => void;
    readonly luteengine_SetLinkRef: (a: number, b: number) => void;
    readonly luteengine_SetMark: (a: number, b: number) => void;
    readonly luteengine_SetParagraphBeginningSpace: (a: number, b: number) => void;
    readonly luteengine_SetPreventEncodeLinkSpace: (a: number, b: number) => void;
    readonly luteengine_SetProtyleMarkNetImg: (a: number, b: number) => void;
    readonly luteengine_SetProtyleWYSIWYG: (a: number, b: number) => [number, number];
    readonly luteengine_SetRenderListStyle: (a: number, b: number) => void;
    readonly luteengine_SetSanitize: (a: number, b: number) => void;
    readonly luteengine_SetSetext: (a: number, b: number) => void;
    readonly luteengine_SetSoftBreak2HardBreak: (a: number, b: number) => void;
    readonly luteengine_SetSpellcheck: (a: number, b: number) => void;
    readonly luteengine_SetSpin: (a: number, b: number) => void;
    readonly luteengine_SetSub: (a: number, b: number) => void;
    readonly luteengine_SetSup: (a: number, b: number) => void;
    readonly luteengine_SetSuperBlock: (a: number, b: number) => void;
    readonly luteengine_SetTabs: (a: number, b: number) => void;
    readonly luteengine_SetTag: (a: number, b: number) => void;
    readonly luteengine_SetTerms: (a: number, b: any) => void;
    readonly luteengine_SetTextMark: (a: number, b: number) => void;
    readonly luteengine_SetToC: (a: number, b: number) => void;
    readonly luteengine_SetUnorderedListMarker: (a: number, b: number, c: number) => void;
    readonly luteengine_SetVditorCodeBlockPreview: (a: number, b: number) => void;
    readonly luteengine_SetVditorHTMLBlockPreview: (a: number, b: number) => void;
    readonly luteengine_SetVditorIR: (a: number, b: number) => void;
    readonly luteengine_SetVditorMathBlockPreview: (a: number, b: number) => void;
    readonly luteengine_SetVditorSV: (a: number, b: number) => void;
    readonly luteengine_SetVditorWYSIWYG: (a: number, b: number) => void;
    readonly luteengine_SetYamlFrontMatter: (a: number, b: number) => void;
    readonly luteengine_Space: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_SpinBlockDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_SpinVditorDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_SpinVditorIRDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_SpinVditorSVDOM: (a: number, b: number, c: number) => [number, number, number, number];
    readonly luteengine_TL2OL: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_TL2UL: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_TextBundle: (a: number, b: number, c: number, d: number, e: number, f: any) => any;
    readonly luteengine_TextBundleStr: (a: number, b: number, c: number, d: number, e: number, f: any) => any;
    readonly luteengine_Tree2BlockDOM: (a: number, b: number, c: any, d: any) => [number, number, number, number];
    readonly luteengine_Tree2HTML: (a: number, b: number, c: any, d: any) => [number, number];
    readonly luteengine_UL2OL: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_UL2TL: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_VditorDOM2HTML: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_VditorDOM2Md: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_VditorIRDOM2HTML: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_VditorIRDOM2Md: (a: number, b: number, c: number) => [number, number];
    readonly luteengine_new: () => number;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
