/* @ts-self-types="./lute_wasm.d.ts" */

/**
 * @param {string} dom
 * @returns {string}
 */
export function BlockDOM2ContentGlobal(dom) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(dom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.BlockDOM2ContentGlobal(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {string} s
 * @returns {string}
 */
export function EChartsMindmapStr(s) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.EChartsMindmapStr(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {string} s
 * @returns {string}
 */
export function EscapeHTMLStr(s) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.EscapeHTMLStr(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {JsNode} node
 * @returns {string}
 */
export function GetHeadingID(node) {
    let deferred1_0;
    let deferred1_1;
    try {
        _assertClass(node, JsNode);
        const ret = wasm.GetHeadingID(node.__wbg_ptr);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @param {JsNode} node
 * @returns {string}
 */
export function GetHeadingIDRaw(node) {
    let deferred1_0;
    let deferred1_1;
    try {
        _assertClass(node, JsNode);
        const ret = wasm.GetHeadingIDRaw(node.__wbg_ptr);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

export class JsNode {
    static __wrap(ptr) {
        const obj = Object.create(JsNode.prototype);
        obj.__wbg_ptr = ptr;
        JsNodeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsNodeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsnode_free(ptr, 0);
    }
    /**
     * @param {JsNode} child
     */
    AppendChild(child) {
        _assertClass(child, JsNode);
        wasm.jsnode_AppendChild(this.__wbg_ptr, child.__wbg_ptr);
    }
    /**
     * @param {string} typ
     * @returns {any}
     */
    ChildByType(typ) {
        const ptr0 = passStringToWasm0(typ, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jsnode_ChildByType(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * @param {string} typ
     * @returns {Array<any>}
     */
    ChildrenByType(typ) {
        const ptr0 = passStringToWasm0(typ, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jsnode_ChildrenByType(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * @returns {string}
     */
    Content() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jsnode_Content(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {JsNode}
     */
    FirstDeepestChild() {
        const ret = wasm.jsnode_FirstDeepestChild(this.__wbg_ptr);
        return JsNode.__wrap(ret);
    }
    /**
     * @param {string} name
     * @returns {string}
     */
    IALAttr(name) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.jsnode_IALAttr(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {JsNode} sibling
     */
    InsertAfter(sibling) {
        _assertClass(sibling, JsNode);
        wasm.jsnode_InsertAfter(this.__wbg_ptr, sibling.__wbg_ptr);
    }
    /**
     * @param {JsNode} sibling
     */
    InsertBefore(sibling) {
        _assertClass(sibling, JsNode);
        wasm.jsnode_InsertBefore(this.__wbg_ptr, sibling.__wbg_ptr);
    }
    /**
     * @returns {boolean}
     */
    IsBlock() {
        const ret = wasm.jsnode_IsBlock(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {JsNode}
     */
    LastDeepestChild() {
        const ret = wasm.jsnode_LastDeepestChild(this.__wbg_ptr);
        return JsNode.__wrap(ret);
    }
    /**
     * @returns {string}
     */
    NextNodeText() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jsnode_NextNodeText(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} typ
     * @returns {boolean}
     */
    ParentIs(typ) {
        const ptr0 = passStringToWasm0(typ, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jsnode_ParentIs(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @returns {string}
     */
    PreviousNodeText() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jsnode_PreviousNodeText(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {string} value
     */
    SetIALAttr(name, value) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.jsnode_SetIALAttr(this.__wbg_ptr, ptr0, len0, ptr1, len1);
    }
    /**
     * @returns {number}
     */
    TextLen() {
        const ret = wasm.jsnode_TextLen(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {string}
     */
    Text() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jsnode_Text(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    TokensStr() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jsnode_TokensStr(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    Unlink() {
        wasm.jsnode_Unlink(this.__wbg_ptr);
    }
    /**
     * 对应 Go 版包装对象的 __internal_object__：字段访问走这里。
     * @returns {any}
     */
    get __internal_object__() {
        const ret = wasm.jsnode___internal_object__(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) JsNode.prototype[Symbol.dispose] = JsNode.prototype.free;

export class JsTree {
    static __wrap(ptr) {
        const obj = Object.create(JsTree.prototype);
        obj.__wbg_ptr = ptr;
        JsTreeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsTreeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jstree_free(ptr, 0);
    }
    /**
     * @returns {string}
     */
    get ID() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jstree_ID(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get Name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jstree_Name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {JsNode}
     */
    Root() {
        const ret = wasm.jstree_Root(this.__wbg_ptr);
        return JsNode.__wrap(ret);
    }
}
if (Symbol.dispose) JsTree.prototype[Symbol.dispose] = JsTree.prototype.free;

/**
 * 对应 Go 版 lute.Lute 的 JS 包装。
 */
export class LuteEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        LuteEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_luteengine_free(ptr, 0);
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2Content(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2Content(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2EscapeMarkerContent(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2EscapeMarkerContent(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2HTML(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2HTML(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2InlineBlockDOM(blockdom) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2InlineBlockDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2Md(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2Md(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2RichHTML(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2RichHTML(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2StdMd(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2StdMd(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {number}
     */
    BlockDOM2TextLen(blockdom) {
        const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_BlockDOM2TextLen(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    BlockDOM2Text(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_BlockDOM2Text(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {JsTree}
     */
    BlockDOM2Tree(blockdom) {
        const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_BlockDOM2Tree(this.__wbg_ptr, ptr0, len0);
        return JsTree.__wrap(ret);
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    Blockquote2Callout(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Blockquote2Callout(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @param {string} level
     * @returns {string}
     */
    Blocks2Hs(blockdom, level) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(level, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Blocks2Hs(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    Blocks2Ps(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Blocks2Ps(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    Callout2Blockquote(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Callout2Blockquote(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    CancelBlockquote(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_CancelBlockquote(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    CancelCallout(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_CancelCallout(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    CancelListRecursively(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_CancelListRecursively(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    CancelList(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_CancelList(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    CancelSuperBlock(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_CancelSuperBlock(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @param {string} target
     * @returns {string}
     */
    ConvertListType(blockdom, target) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(target, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_ConvertListType(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {string} markdown
     * @returns {string}
     */
    FormatStr(name, markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_FormatStr(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {Uint8Array} markdown
     * @returns {Uint8Array}
     */
    Format(name, markdown) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(markdown, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_Format(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v3;
    }
    /**
     * @returns {object}
     */
    GetEmojis() {
        const ret = wasm.luteengine_GetEmojis(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {string}
     */
    GetLinkBase() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.luteengine_GetLinkBase(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} s
     * @returns {string}
     */
    GetLinkDest(s) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_GetLinkDest(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {object}
     */
    GetTerms() {
        const ret = wasm.luteengine_GetTerms(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {string} html
     * @returns {string}
     */
    HTML2BlockDOM(html) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_HTML2BlockDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} dom
     * @returns {Array<any>}
     */
    HTML2Markdown(dom) {
        const ptr0 = passStringToWasm0(dom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_HTML2Markdown(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} dom
     * @returns {string}
     */
    HTML2Md(dom) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(dom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_HTML2Md(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} dom
     * @returns {string}
     */
    HTML2Text(dom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(dom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_HTML2Text(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} html
     * @returns {JsTree}
     */
    HTML2Tree(html) {
        const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_HTML2Tree(this.__wbg_ptr, ptr0, len0);
        return JsTree.__wrap(ret);
    }
    /**
     * @param {string} html
     * @returns {string}
     */
    HTML2VditorDOM(html) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_HTML2VditorDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} html
     * @returns {string}
     */
    HTML2VditorIRDOM(html) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_HTML2VditorIRDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} html
     * @returns {string}
     */
    HTML2VditorSVDOM(html) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_HTML2VditorSVDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * 对应 Go 版 Lute.HTMLNode2Tree。JS 侧没有 html.Node 包装，这里接受 HTML 字符串。
     * @param {string} node
     * @returns {JsTree}
     */
    HTMLNode2Tree(node) {
        const ptr0 = passStringToWasm0(node, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_HTMLNode2Tree(this.__wbg_ptr, ptr0, len0);
        return JsTree.__wrap(ret);
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    InlineMd2BlockDOM(markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_InlineMd2BlockDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} s
     * @returns {boolean}
     */
    IsValidLinkDest(s) {
        const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_IsValidLinkDest(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} name
     * @param {string} markdown
     * @returns {string}
     */
    MarkdownStr(name, markdown) {
        let deferred4_0;
        let deferred4_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_MarkdownStr(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {Uint8Array} markdown
     * @returns {Uint8Array}
     */
    Markdown(name, markdown) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(markdown, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_Markdown(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v3;
    }
    /**
     * @param {string} markdown
     * @param {boolean} reserve_empty_paragraph
     * @returns {Array<any>}
     */
    Md2BlockDOMTree(markdown, reserve_empty_paragraph) {
        const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_Md2BlockDOMTree(this.__wbg_ptr, ptr0, len0, reserve_empty_paragraph);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} markdown
     * @param {boolean} reserve_empty_paragraph
     * @returns {string}
     */
    Md2BlockDOMWithAutoLink(markdown, reserve_empty_paragraph) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Md2BlockDOMWithAutoLink(this.__wbg_ptr, ptr0, len0, reserve_empty_paragraph);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @param {boolean} reserve_empty_paragraph
     * @returns {string}
     */
    Md2BlockDOM(markdown, reserve_empty_paragraph) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Md2BlockDOM(this.__wbg_ptr, ptr0, len0, reserve_empty_paragraph);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    Md2HTML(markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Md2HTML(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    Md2VditorDOM(markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Md2VditorDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    Md2VditorIRDOM(markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Md2VditorIRDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    Md2VditorSVDOM(markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Md2VditorSVDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {JsNode} node
     */
    MergeSameSpan(node) {
        _assertClass(node, JsNode);
        wasm.luteengine_MergeSameSpan(this.__wbg_ptr, node.__wbg_ptr);
    }
    /**
     * @param {JsNode} node
     */
    MergeSameTextMark(node) {
        _assertClass(node, JsNode);
        wasm.luteengine_MergeSameTextMark(this.__wbg_ptr, node.__wbg_ptr);
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    OL2TL(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_OL2TL(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    OL2UL(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_OL2UL(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {string} markdown
     * @returns {string}
     */
    ProtylePreviewStr(name, markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_ProtylePreviewStr(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {Uint8Array} markdown
     * @returns {Uint8Array}
     */
    ProtylePreview(name, markdown) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(markdown, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_ProtylePreview(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v3;
    }
    /**
     * @param {object} emojis
     */
    PutEmojis(emojis) {
        wasm.luteengine_PutEmojis(this.__wbg_ptr, emojis);
    }
    /**
     * @param {object} terms
     */
    PutTerms(terms) {
        wasm.luteengine_PutTerms(this.__wbg_ptr, terms);
    }
    /**
     * @param {string} s
     * @returns {string}
     */
    RemoveEmoji(s) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_RemoveEmoji(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    RenderEChartsJSON(markdown) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_RenderEChartsJSON(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    RenderJSON(markdown) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_RenderJSON(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    RenderKityMinderJSON(markdown) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_RenderKityMinderJSON(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {JsNode} node
     * @returns {string}
     */
    RenderNodeBlockDOM(node) {
        let deferred2_0;
        let deferred2_1;
        try {
            _assertClass(node, JsNode);
            const ret = wasm.luteengine_RenderNodeBlockDOM(this.__wbg_ptr, node.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {boolean} b
     */
    SetArbitraryTaskListItemMarker(b) {
        wasm.luteengine_SetArbitraryTaskListItemMarker(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetAutoSpace(b) {
        const ret = wasm.luteengine_SetAutoSpace(this.__wbg_ptr, b);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {boolean} b
     */
    SetBlockRef(b) {
        wasm.luteengine_SetBlockRef(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetCallout(b) {
        wasm.luteengine_SetCallout(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetChineseParagraphBeginningSpace(b) {
        wasm.luteengine_SetChineseParagraphBeginningSpace(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetCodeSyntaxHighlightDetectLang(b) {
        wasm.luteengine_SetCodeSyntaxHighlightDetectLang(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetCodeSyntaxHighlightInlineStyle(b) {
        wasm.luteengine_SetCodeSyntaxHighlightInlineStyle(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetCodeSyntaxHighlightLineNum(b) {
        wasm.luteengine_SetCodeSyntaxHighlightLineNum(this.__wbg_ptr, b);
    }
    /**
     * @param {string} name
     */
    SetCodeSyntaxHighlightStyleName(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetCodeSyntaxHighlightStyleName(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetCodeSyntaxHighlight(b) {
        wasm.luteengine_SetCodeSyntaxHighlight(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetCustomBlock(b) {
        wasm.luteengine_SetCustomBlock(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetDataImage(b) {
        wasm.luteengine_SetDataImage(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetDataTask(b) {
        wasm.luteengine_SetDataTask(this.__wbg_ptr, b);
    }
    /**
     * @param {string} site
     */
    SetEmojiSite(site) {
        const ptr0 = passStringToWasm0(site, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetEmojiSite(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetEmoji(b) {
        wasm.luteengine_SetEmoji(this.__wbg_ptr, b);
    }
    /**
     * @param {object} emojis
     */
    SetEmojis(emojis) {
        wasm.luteengine_SetEmojis(this.__wbg_ptr, emojis);
    }
    /**
     * @param {boolean} b
     */
    SetEnsureListItemParagraph(b) {
        wasm.luteengine_SetEnsureListItemParagraph(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetExportNormalizeTaskListMarker(b) {
        wasm.luteengine_SetExportNormalizeTaskListMarker(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetFileAnnotationRef(b) {
        wasm.luteengine_SetFileAnnotationRef(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetFixTermTypo(b) {
        wasm.luteengine_SetFixTermTypo(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetFootnotes(b) {
        wasm.luteengine_SetFootnotes(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetFullWidthStrikethrough(b) {
        wasm.luteengine_SetFullWidthStrikethrough(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetGFMAutoLink(b) {
        wasm.luteengine_SetGFMAutoLink(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetGFMStrikethrough1(b) {
        wasm.luteengine_SetGFMStrikethrough1(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetGFMStrikethrough(b) {
        wasm.luteengine_SetGFMStrikethrough(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetGFMTable(b) {
        wasm.luteengine_SetGFMTable(this.__wbg_ptr, b);
    }
    /**
     * @param {string} _class
     */
    SetGFMTaskListItemClass(_class) {
        const ptr0 = passStringToWasm0(_class, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetGFMTaskListItemClass(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetGFMTaskListItem(b) {
        wasm.luteengine_SetGFMTaskListItem(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetGitConflict(b) {
        wasm.luteengine_SetGitConflict(this.__wbg_ptr, b);
    }
    /**
     * @param {Array<any>} attrs
     */
    SetHTML2MarkdownAttrs(attrs) {
        wasm.luteengine_SetHTML2MarkdownAttrs(this.__wbg_ptr, attrs);
    }
    /**
     * @param {boolean} b
     */
    SetHTMLTag2TextMark(b) {
        wasm.luteengine_SetHTMLTag2TextMark(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetHeadingAnchor(b) {
        wasm.luteengine_SetHeadingAnchor(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetHeadingID(b) {
        wasm.luteengine_SetHeadingID(this.__wbg_ptr, b);
    }
    /**
     * @param {string} data_src
     */
    SetImageLazyLoading(data_src) {
        const ptr0 = passStringToWasm0(data_src, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetImageLazyLoading(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetImgPathAllowSpace(b) {
        wasm.luteengine_SetImgPathAllowSpace(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetImgTag(b) {
        wasm.luteengine_SetImgTag(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetIndentCodeBlock(b) {
        wasm.luteengine_SetIndentCodeBlock(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetInlineAsterisk(b) {
        wasm.luteengine_SetInlineAsterisk(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetInlineMathAllowDigitAfterOpenMarker(b) {
        wasm.luteengine_SetInlineMathAllowDigitAfterOpenMarker(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetInlineMath(b) {
        wasm.luteengine_SetInlineMath(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetInlineUnderscore(b) {
        wasm.luteengine_SetInlineUnderscore(this.__wbg_ptr, b);
    }
    /**
     * 对应 Go 版 Lute.SetJSRenderers。
     * @param {any} options
     */
    SetJSRenderers(options) {
        const ret = wasm.luteengine_SetJSRenderers(this.__wbg_ptr, options);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {boolean} b
     */
    SetKramdownBlockIAL(b) {
        const ret = wasm.luteengine_SetKramdownBlockIAL(this.__wbg_ptr, b);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} name
     */
    SetKramdownIALIDRenderName(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetKramdownIALIDRenderName(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetKramdownIAL(b) {
        wasm.luteengine_SetKramdownIAL(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetKramdownSpanIAL(b) {
        wasm.luteengine_SetKramdownSpanIAL(this.__wbg_ptr, b);
    }
    /**
     * @param {string} base
     */
    SetLinkBase(base) {
        const ptr0 = passStringToWasm0(base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetLinkBase(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} prefix
     */
    SetLinkPrefix(prefix) {
        const ptr0 = passStringToWasm0(prefix, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetLinkPrefix(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetLinkRef(b) {
        wasm.luteengine_SetLinkRef(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetMark(b) {
        wasm.luteengine_SetMark(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetParagraphBeginningSpace(b) {
        wasm.luteengine_SetParagraphBeginningSpace(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetPreventEncodeLinkSpace(b) {
        wasm.luteengine_SetPreventEncodeLinkSpace(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetProtyleMarkNetImg(b) {
        wasm.luteengine_SetProtyleMarkNetImg(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetProtyleWYSIWYG(b) {
        const ret = wasm.luteengine_SetProtyleWYSIWYG(this.__wbg_ptr, b);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {boolean} b
     */
    SetRenderListStyle(b) {
        wasm.luteengine_SetRenderListStyle(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSanitize(b) {
        wasm.luteengine_SetSanitize(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSetext(b) {
        wasm.luteengine_SetSetext(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSoftBreak2HardBreak(b) {
        wasm.luteengine_SetSoftBreak2HardBreak(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSpellcheck(b) {
        wasm.luteengine_SetSpellcheck(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSpin(b) {
        wasm.luteengine_SetSpin(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSub(b) {
        wasm.luteengine_SetSub(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSup(b) {
        wasm.luteengine_SetSup(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetSuperBlock(b) {
        wasm.luteengine_SetSuperBlock(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetTabs(b) {
        wasm.luteengine_SetTabs(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetTag(b) {
        wasm.luteengine_SetTag(this.__wbg_ptr, b);
    }
    /**
     * @param {object} terms
     */
    SetTerms(terms) {
        wasm.luteengine_SetTerms(this.__wbg_ptr, terms);
    }
    /**
     * @param {boolean} b
     */
    SetTextMark(b) {
        wasm.luteengine_SetTextMark(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetToC(b) {
        wasm.luteengine_SetToC(this.__wbg_ptr, b);
    }
    /**
     * @param {string} marker
     */
    SetUnorderedListMarker(marker) {
        const ptr0 = passStringToWasm0(marker, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.luteengine_SetUnorderedListMarker(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} b
     */
    SetVditorCodeBlockPreview(b) {
        wasm.luteengine_SetVditorCodeBlockPreview(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetVditorHTMLBlockPreview(b) {
        wasm.luteengine_SetVditorHTMLBlockPreview(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetVditorIR(b) {
        wasm.luteengine_SetVditorIR(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetVditorMathBlockPreview(b) {
        wasm.luteengine_SetVditorMathBlockPreview(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetVditorSV(b) {
        wasm.luteengine_SetVditorSV(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetVditorWYSIWYG(b) {
        wasm.luteengine_SetVditorWYSIWYG(this.__wbg_ptr, b);
    }
    /**
     * @param {boolean} b
     */
    SetYamlFrontMatter(b) {
        wasm.luteengine_SetYamlFrontMatter(this.__wbg_ptr, b);
    }
    /**
     * @param {string} text
     * @returns {string}
     */
    Space(text) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_Space(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    SpinBlockDOM(blockdom) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_SpinBlockDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} vhtml
     * @returns {string}
     */
    SpinVditorDOM(vhtml) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(vhtml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_SpinVditorDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} vhtml
     * @returns {string}
     */
    SpinVditorIRDOM(vhtml) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(vhtml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_SpinVditorIRDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} markdown
     * @returns {string}
     */
    SpinVditorSVDOM(markdown) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_SpinVditorSVDOM(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    TL2OL(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_TL2OL(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    TL2UL(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_TL2UL(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {string} markdown
     * @param {Array<any>} link_prefixes
     * @returns {Array<any>}
     */
    TextBundleStr(name, markdown, link_prefixes) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(markdown, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_TextBundleStr(this.__wbg_ptr, ptr0, len0, ptr1, len1, link_prefixes);
        return ret;
    }
    /**
     * @param {string} name
     * @param {Uint8Array} markdown
     * @param {Array<any>} link_prefixes
     * @returns {Array<any>}
     */
    TextBundle(name, markdown, link_prefixes) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(markdown, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.luteengine_TextBundle(this.__wbg_ptr, ptr0, len0, ptr1, len1, link_prefixes);
        return ret;
    }
    /**
     * @param {JsTree} tree
     * @param {any} _options
     * @param {any} _parse_options
     * @returns {string}
     */
    Tree2BlockDOM(tree, _options, _parse_options) {
        let deferred2_0;
        let deferred2_1;
        try {
            _assertClass(tree, JsTree);
            const ret = wasm.luteengine_Tree2BlockDOM(this.__wbg_ptr, tree.__wbg_ptr, _options, _parse_options);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {JsTree} tree
     * @param {any} _options
     * @param {any} _parse_options
     * @returns {string}
     */
    Tree2HTML(tree, _options, _parse_options) {
        let deferred1_0;
        let deferred1_1;
        try {
            _assertClass(tree, JsTree);
            const ret = wasm.luteengine_Tree2HTML(this.__wbg_ptr, tree.__wbg_ptr, _options, _parse_options);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    UL2OL(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_UL2OL(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} blockdom
     * @returns {string}
     */
    UL2TL(blockdom) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(blockdom, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_UL2TL(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} vhtml
     * @returns {string}
     */
    VditorDOM2HTML(vhtml) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(vhtml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_VditorDOM2HTML(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} vhtml
     * @returns {string}
     */
    VditorDOM2Md(vhtml) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(vhtml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_VditorDOM2Md(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} vhtml
     * @returns {string}
     */
    VditorIRDOM2HTML(vhtml) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(vhtml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_VditorIRDOM2HTML(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} vhtml
     * @returns {string}
     */
    VditorIRDOM2Md(vhtml) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(vhtml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.luteengine_VditorIRDOM2Md(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    constructor() {
        const ret = wasm.luteengine_new();
        this.__wbg_ptr = ret;
        LuteEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) LuteEngine.prototype[Symbol.dispose] = LuteEngine.prototype.free;

/**
 * @returns {string}
 */
export function NewNodeID() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.NewNodeID();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @param {string} s
 * @returns {string}
 */
export function Sanitize(s) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.Sanitize(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {string} s
 * @returns {string}
 */
export function UnEscapeHTMLStr(s) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.UnEscapeHTMLStr(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @returns {string}
 */
export function luteVersion() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.luteVersion();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_is_function_1f9d30630b8b1d3d: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_3c45d4f2dde4e749: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_number_get_2e0e7dee9f701a71: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_0380ccaa2f57f0d9: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_41e9ee4f547fc59a: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_1875a20c43a36133: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.call(arg1, arg2, arg3);
            return ret;
        }, arguments); },
        __wbg_get_31af05bd4842a84f: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_6c896e0571ddae51: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_288889d017702237: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_isArray_e15a2ff68ffdbef2: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_jsnode_new: function(arg0) {
            const ret = JsNode.__wrap(arg0);
            return ret;
        },
        __wbg_jstree_new: function(arg0) {
            const ret = JsTree.__wrap(arg0);
            return ret;
        },
        __wbg_keys_440be172f17c0265: function(arg0) {
            const ret = Object.keys(arg0);
            return ret;
        },
        __wbg_length_d4bdea10311bd9cf: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_343a093a3c2ffb4e: function(arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_617a8cdb8bb1130e: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_ee2291f50781bf1d: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_now_aa4ccb83129e9e55: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_push_2baf45db356cf468: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_set_145a351398b48c65: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./lute_wasm_bg.js": import0,
    };
}

const JsNodeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsnode_free(ptr, 1));
const JsTreeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jstree_free(ptr, 1));
const LuteEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_luteengine_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('lute_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
