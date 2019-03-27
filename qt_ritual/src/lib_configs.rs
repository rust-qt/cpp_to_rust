//! Generator configurations specific for each Qt module.

use crate::detect_signals_and_slots::detect_signals_and_slots;
use crate::doc_parser::parse_docs;
use crate::fix_header_names::fix_header_names;
use crate::slot_wrappers::add_slot_wrappers;
use crate::versions;
use log::info;
use qt_ritual_common::{all_crate_names, get_full_build_config, lib_dependencies, lib_folder_name};
use ritual::config::CrateProperties;
use ritual::config::{Config, GlobalConfig};
use ritual::cpp_data::CppPath;
use ritual::cpp_ffi_data::CppFfiFunctionKind;
use ritual::cpp_type::CppType;
use ritual::rust_info::{NameType, RustPathScope};
use ritual::rust_type::RustPath;
use ritual_common::cpp_build_config::CppLibraryType;
use ritual_common::cpp_build_config::{CppBuildConfigData, CppBuildPaths};
use ritual_common::errors::{bail, Result, ResultExt};
use ritual_common::file_utils::repo_dir_path;
use ritual_common::target;
use ritual_common::toml;
use std::path::PathBuf;

/*
/// Helper method to blacklist all methods of `QList<T>` template instantiation that
/// don't work if `T` doesn't have `operator==`. `types` is list of such `T` types.
fn exclude_qlist_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(
  config: &mut Config,
  types: I,
) {
  let types = types.into_iter().map(|x| x.as_ref().to_string()).collect_vec();
  config.add_cpp_ffi_generator_filter(move |method| {
    if let Some(info) = &method.class_membership {
      if info.class_type.name == "QList" {
        let args = info
          .class_type
          .template_arguments
          .as_ref()
          .with_context(|| "failed to get QList args")?;
        let arg = args.get(0).with_context(|| "failed to get QList arg")?;
        let arg_text = arg.to_cpp_pseudo_code();
        if types.iter().any(|x| x == &arg_text) {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "indexOf" | "lastIndexOf" | "contains" | "startsWith"
            | "endsWith" | "removeOne" | "removeAll" | "value" | "toVector" | "toSet" => {
              return Ok(false)
            }
            "count" => {
              if method.arguments.len() == 1 {
                return Ok(false);
              }
            }
            _ => {}
          }
        }
      }
    }
    Ok(true)
  });
}

/// Helper method to blacklist all methods of `QVector<T>` template instantiation that
/// don't work if `T` doesn't have `operator==`. `types` is list of such `T` types.
fn exclude_qvector_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(
  config: &mut Config,
  types: I,
) {
  let types = types.into_iter().map(|x| x.as_ref().to_string()).collect_vec();
  config.add_cpp_ffi_generator_filter(move |method| {
    if let Some(info) = &method.class_membership {
      if info.class_type.name == "QVector" {
        let args = info
          .class_type
          .template_arguments
          .as_ref()
          .with_context(|| "failed to get QVector args")?;
        let arg = args.get(0).with_context(|| "failed to get QVector arg")?;
        let arg_text = arg.to_cpp_pseudo_code();
        if types.iter().any(|x| x == &arg_text) {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "indexOf" | "lastIndexOf" | "contains" | "startsWith"
            | "endsWith" | "removeOne" | "removeAll" | "toList" => return Ok(false),
            "count" => {
              if method.arguments.len() == 1 {
                return Ok(false);
              }
            }
            _ => {}
          }
        }
      }
    }
    Ok(true)
  });
}

/// List of QtCore identifiers that should be blacklisted.
#[cfg_attr(rustfmt, rustfmt_skip)]
fn core_cpp_parser_blocked_names() -> Vec<&'static str> {
  vec![
    "QAbstractConcatenable", "QAlgorithmsPrivate", "QArrayData",
    "QArrayDataPointer", "QArrayDataPointerRef", "QAtomicAdditiveType",
    "QAtomicInt", "QAtomicInteger", "QAtomicOps", "QAtomicPointer",
    "QBasicAtomicInteger", "QBasicAtomicInteger", "QBasicAtomicPointer",
    "QBitArray::detach", "QBitArray::isDetached", "QByteArray::detach",
    "QByteArray::isDetached", "QByteArray::isSharedWith", "QByteArrayDataPtr",
    "QConcatenable", "QConstOverload", "QContiguousCache::detach",
    "QContiguousCache::isDetached", "QContiguousCache::setSharable",
    "QContiguousCacheData", "QContiguousCacheTypedData", "QEnableSharedFromThis",
    "QException", "QFlag", "QForeachContainer", "QGenericAtomicOps",
    "QHash::detach", "QHash::isDetached", "QHash::setSharable", "QHashData",
    "QHashDummyValue", "QHashNode", "QHashNode", "QIncompatibleFlag", "QInternal",
    "QJsonValuePtr", "QJsonValueRefPtr", "QLinkedList::detach",
    "QLinkedList::isDetached", "QLinkedList::isSharedWith",
    "QLinkedList::setSharable", "QLinkedListData", "QLinkedListNode",
    "QList::detach", "QList::detachShared", "QList::isDetached",
    "QList::isSharedWith", "QList::setSharable", "QListData", "QMap::detach",
    "QMap::isDetached", "QMap::isSharedWith", "QMap::setSharable", "QMapData",
    "QMapDataBase", "QMapNode", "QMapNodeBase", "QMessageLogContext::copy",
    "QMetaObject::Connection::isConnected_helper", "QMetaTypeId", "QMetaTypeId2",
    "QNoDebug", "QNonConstOverload", "QObject::registerUserData", "QObjectData",
    "QObjectUserData", "QObjectUserData", "QPersistentModelIndex::internalId",
    "QPersistentModelIndex::internalPointer", "QScopedPointerArrayDeleter",
    "QScopedPointerDeleter", "QScopedPointerObjectDeleteLater",
    "QScopedPointerPodDeleter", "QSet::detach", "QSet::isDetached",
    "QSet::setSharable", "QString::Null", "QString::detach",
    "QString::isDetached", "QString::isSharedWith", "QString::isSimpleText",
    "QString::vasprintf", "QString::vsprintf", "QStringDataPtr",
    "QThreadStorageData", "QTypeInfo", "QTypeInfoMerger", "QTypeInfoQuery",
    "QTypedArrayData", "QUnhandledException", "QUrl::detach", "QUrl::isDetached",
    "QUrlQuery::isDetached", "QVariant::Handler", "QVariant::Private",
    "QVariant::PrivateShared", "QVariant::constData", "QVariant::data",
    "QVariant::detach", "QVariant::isDetached", "QVariantComparisonHelper",
    "QVector::detach", "QVector::isDetached", "QVector::isSharedWith",
    "QVector::setSharable", "Qt::Initialization", "QtGlobalStatic",
    "QtMetaTypePrivate", "QtPrivate", "QtSharedPointer", "QtStringBuilder",
    "_GUID", "qBadAlloc", "qErrnoWarning", "qFlagLocation", "qGreater", "qLess",
    "qMapLessThanKey", "qSharedBuild", "qYouForgotTheQ_OBJECT_Macro",
    "qbswap_helper", "qobject_interface_iid", "qt_QMetaEnum_debugOperator",
    "qt_QMetaEnum_flagDebugOperator", "qt_assert", "qt_assert_x",
    "qt_check_for_QGADGET_macro", "qt_check_for_QOBJECT_macro",
    "qt_check_pointer", "qt_hash", "qt_message_output", "qt_metacall",
    "qt_metacast", "qt_noop", "qt_qFindChild_helper", "qt_qFindChildren_helper",
    "qt_sharedpointer_cast_check", "qvsnprintf", "std",
    "qThreadStorage_deleteData", "QStringBuilderCommon", "QStringBuilderBase", "QStringBuilder",
    "QFutureInterfaceBase", "QFutureInterface", "QFutureWatcherBase", "QFutureWatcher"
  ]
}
*/
/// QtCore specific configuration.
fn core_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_core"),
                prefix: None,
            }));
        }
        Ok(None)
    });

    /*config.set_movable_types_hook(|path| {
        let string = path.to_templateless_string();
        let movable = &[
            "QMetaObject::Connection",
            "QStringRef",
            "QXmlStreamAttribute",
            "QStack",
            "QFileInfo",
            "QString",
            "QCborParserError",
            "QCborError",
            "QVectorIterator",
            "QJsonObject::iterator",
            "QFuture",
            "QStaticPlugin",
            "QAssociativeIterable::const_iterator",
            "QJsonArray::const_iterator",
            "QArgument",
            "QXmlStreamWriter",
            "QDebugStateSaver",
            "QJsonValue",
            "QByteArray",
            "QMutableHashIterator",
            "QOperatingSystemVersion",
            "QSize",
            "QLibraryInfo",
            "QTimeZone::OffsetData",
            "QVariant",
            "QLineF",
            "QUrlTwoFlags",
            "QUrl",
            "QMargins",
            "QDir",
            "QEasingCurve",
            "QUrlQuery",
            "QList",
            "QStorageInfo",
            "QXmlStreamNamespaceDeclaration",
            "QDataStream",
            "QRect",
            "QCborMap",
            "QLocale",
            "QSizeF",
            "QCborArray",
            "QPoint",
            "QCharRef",
            "QMimeType",
            "QCborValue",
            "QRegExp",
            "QCommandLineOption",
            "QUuid",
            "QCborArray::ConstIterator",
            "QMetaMethod",
            "QDate",
            "QJsonValueRef",
            "QSequentialIterable::const_iterator",
            "QCborArray::Iterator",
            "QDeadlineTimer",
            "QJsonArray::iterator",
            "QRegularExpression",
            "QCollatorSortKey",
            "QTimeZone",
            "QCborMap::ConstIterator",
            "QMarginsF",
            "QJsonDocument",
            "QLine",
            "QItemSelectionRange",
            "QVersionNumber",
            "QMap",
            "QBitArray",
            "QModelIndex",
            "QJsonObject::const_iterator",
            "QTime",
            "QDateTime",
            "QPersistentModelIndex",
            "QVector",
            "QStringView",
            "QLinkedList",
            "QProcessEnvironment",
            "QRectF",
            "QDebug",
            "QHash",
            "QLatin1String",
            "QPointer",
            "QStringList",
            "QCborMap::Iterator",
            "QSet",
            "QPair",
            "QByteArrayMatcher",
            "QSysInfo",
            "QMultiHash",
            "QCollator",
            "QCborStreamReader::StringResult",
            "QBasicMutex",
            "QMutableVectorIterator",
            "QContiguousCache",
            "QListIterator",
            "QRegularExpressionMatchIterator",
            "QMessageAuthenticationCode",
            "QResource",
            "QThreadStorage",
            "QLatin1Char",
            "QMultiMap",
            "QTextBoundaryFinder",
            "QMessageLogger",
            "QMutableLinkedListIterator",
            "QCborStreamReader",
            "QJsonObject",
            "QReturnArgument",
            "QMutex",
            "QElapsedTimer",
            "QItemSelection",
            "QSignalBlocker",
            "QWriteLocker",
            "QXmlStreamAttributes",
            "QMutableMapIterator",
            "QChar",
            "QStandardPaths",
            "QMetaType",
            "QListSpecialMethods",
            "QRegularExpressionMatch",
            "QAssociativeIterable",
            "QStringBuilderBase",
            "QLinkedListIterator",
            "QConcatenable",
            "QDirIterator",
            "QSemaphore",
            "QMimeDatabase",
            "QXmlStreamStringRef",
            "QMutableSetIterator",
            "QSetIterator",
            "QQueue",
            "QPointF",
            "QMetaProperty",
            "QMetaEnum",
            "QCryptographicHash",
            "QJsonParseError",
            "QFutureSynchronizer",
            "QFutureIterator",
            "QSemaphoreReleaser",
            "QXmlStreamNotationDeclaration",
            "QCborValueRef",
            "QStringMatcher",
            "QJsonArray",
            "QBitRef",
            "QMapIterator",
            "QCache",
            "QMutexLocker",
            "QByteRef",
            "QSequentialIterable",
            "QMetaClassInfo",
            "QCommandLineParser",
            "QTemporaryDir",
            "QTextEncoder",
            "QCborStreamWriter",
            "QKeyValueIterator",
            "QReadWriteLock",
            "QEventLoopLocker",
            "QTextDecoder",
            "QReadLocker",
            "QLoggingCategory",
            "QSystemSemaphore",
            "qfloat16",
            "QStaticByteArrayMatcherBase",
            "QStringBuilderCommon",
            "QMutableListIterator",
            "QHashIterator",
            "QWaitCondition",
            "QTypeInfoQuery",
            "QBasicTimer",
            "QXmlStreamEntityDeclaration",
            "QMessageLogContext",
            "QLockFile",
            "QAbstractEventDispatcher::TimerInfo",
            "QTypeInfo",
            "QTextStreamManipulator",
        ];

        let immovable = &[
            "QAbstractTableModel",
            "QEvent",
            "QLibrary",
            "QIdentityProxyModel",
            "QAbstractState",
            "QXmlStreamReader",
            "QTextCodec",
            "QPauseAnimation",
            "QObject",
            "QAnimationGroup",
            "QTimeLine",
            "QStateMachine::SignalEvent",
            "QFutureWatcher",
            "QSocketNotifier",
            "QFactoryInterface",
            "QThread",
            "QTransposeProxyModel",
            "QObjectCleanupHandler",
            "QSettings",
            "QEventLoop",
            "QDynamicPropertyChangeEvent",
            "QAbstractTransition",
            "QAbstractNativeEventFilter",
            "QFileSelector",
            "QFinalState",
            "QUnhandledException",
            "QTemporaryFile",
            "QAbstractEventDispatcher",
            "QPluginLoader",
            "QMimeData",
            "QPropertyAnimation",
            "QBuffer",
            "QStateMachine",
            "QItemSelectionModel",
            "QSequentialAnimationGroup",
            "QEventTransition",
            "QFileDevice",
            "QSharedMemory",
            "QSaveFile",
            "QAbstractListModel",
            "QVariantAnimation",
            "QSignalTransition",
            "QFutureWatcherBase",
            "QStringListModel",
            "QAbstractItemModel",
            "QStateMachine::WrappedEvent",
            "QSortFilterProxyModel",
            "QAnimationDriver",
            "QState",
            "QHistoryState",
            "QTextStream",
            "QParallelAnimationGroup",
            "QTimerEvent",
            "QAbstractAnimation",
            "QDeferredDeleteEvent",
            "QThreadPool",
            "QChildEvent",
            "QXmlStreamEntityResolver",
            "QConcatenateTablesProxyModel",
            "QFileSystemWatcher",
            "QProcess",
            "QRunnable",
            "QTranslator",
            "QSignalMapper",
            "QFile",
            "QIODevice",
            "QCoreApplication",
            "QAbstractProxyModel",
            "QTimer",
            "QRandomGenerator",
            "QRandomGenerator64",
            "QMetaObject",
        ];
        if movable.contains(&string.as_str()) {
            return Ok(MovableTypesHookOutput::Movable);
        }
        if immovable.contains(&string.as_str()) {
            return Ok(MovableTypesHookOutput::Immovable);
        }
        Ok(MovableTypesHookOutput::Unknown)
    });*/

    config.set_cpp_parser_path_hook(|path| {
        let string = path.to_templateless_string();
        let blocked = &[
            // Qt internals, not intended for direct use
            "QtPrivate",
            "QAlgorithmsPrivate",
            "QtMetaTypePrivate",
            "QInternal",
            "qFlagLocation",
            "QArrayData",
            "QTypedArrayData",
            "QStaticByteArrayData",
            "QListData",
            "QObjectData",
            "QObjectUserData",
            "QMapNodeBase",
            "QMapNode",
            "QMapDataBase",
            "QMapData",
            "QHashData",
            "QHashDummyValue",
            "QHashNode",
            "QContiguousCacheData",
            "QLinkedListData",
            "QLinkedListNode",
            "QThreadStorageData",
            "QVariant::Private",
            "QVariant::PrivateShared",
            "QByteArrayDataPtr",
            "QStringDataPtr",
            "QArrayDataPointer",
            "QArrayDataPointerRef",
            "QMetaTypeId",
            "QMetaTypeId2",
            "QVariantComparisonHelper",
            "QtStringBuilder",
            "QVariant::Handler",
            // deprecated
            "qGreater",
            "qLess",
            "QString::Null",
            // undocumented, does nothing
            "qt_noop",
            "QNoDebug",
            // undocumented, unknown purpose
            "qTerminate",
            "qt_error_string",
            "QFutureInterfaceBase",
            "QFutureInterfaceBase",
            "Qt::Initialization",
            "QAbstractConcatenable",
            "QTextCodec::ConverterState",
            "QJsonValuePtr",
            "QJsonValueRefPtr",
            "QTypeInfoMerger",
            // for Q_ASSERT, Q_ASSERT_X macros, no need to access this from Rust
            "qt_assert",
            "qt_assert_x",
            // for Q_CHECK_PTR macro, no need to access this from Rust
            "qt_check_pointer",
            "q_check_ptr",
            // atomic operations, useless in Rust
            "QGenericAtomicOps",
            "QAtomicTraits",
            "QAtomicOps",
            "QBasicAtomicInteger",
            "QBasicAtomicPointer",
            "qAtomicAssign",
            "qAtomicDetach",
            "QAtomicAdditiveType",
            "QAtomicInt",
            "QAtomicPointer",
            "QAtomicInteger",
            // BE/LE integers, useless in Rust
            "QSpecialInteger",
            "QBigEndianStorageType",
            "QLittleEndianStorageType",
            // works on overloading, can't be useful in Rust
            "Qt::qt_getEnumName",
            // reimplemented in Rust
            "QFlags",
            "QFlag",
            "QIncompatibleFlag",
            // not useful in Rust
            "QtSharedPointer",
            "QSharedPointer",
            "QWeakPointer",
            "QEnableSharedFromThis",
            "QScopedArrayPointer",
            // throws exception, so useless here
            "qBadAlloc",
            // requires user class templates, so useless here
            "QSharedDataPointer",
            "QExplicitlySharedDataPointer",
            "QSharedData",
            "QScopeGuard",
            "QScopedValueRollback",
            "QScopedPointer",
            "QScopedPointerObjectDeleteLater",
            "QScopedPointerPodDeleter",
            "QScopedPointerDeleter",
            "QScopedPointerArrayDeleter",
            "QGenericArgument",
            "QGenericReturnArgument",
            "QNonConstOverload",
            "QConstOverload",
            // global functions that redirects to member functions
            "swap",
        ];
        if blocked.contains(&string.as_str()) {
            return Ok(false);
        }

        Ok(true)
    });

    // TODO: replace QVariant::Type with QMetaType::Type?
    //config.add_cpp_parser_blocked_names(core_cpp_parser_blocked_names());
    //config.add_cpp_parser_blocked_names(vec!["QtMetaTypePrivate", "QtPrivate"]);

    // TODO: the following items should be conditionally available on Windows;
    /*config.add_cpp_parser_blocked_names(vec![
      "QWinEventNotifier",
      "QProcess::CreateProcessArguments",
      "QProcess::nativeArguments",
      "QProcess::setNativeArguments",
      "QProcess::createProcessArgumentsModifier",
      "QProcess::setCreateProcessArgumentsModifier",
      "QAbstractEventDispatcher::registerEventNotifier",
      "QAbstractEventDispatcher::unregisterEventNotifier",
    ]);*/

    // QProcess::pid returns different types on different platforms,
    // but this method is obsolete anyway
    //config.add_cpp_parser_blocked_names(vec![CppPath::from_good_str("QProcess::pid")]);
    /*
    exclude_qvector_eq_based_methods(config, &["QStaticPlugin", "QTimeZone::OffsetData"]);
    exclude_qlist_eq_based_methods(
      config,
      &["QAbstractEventDispatcher::TimerInfo", "QCommandLineOption"],
    );

    config.set_types_allocation_place(
      CppTypeAllocationPlace::Stack,
      vec![
        "QAssociativeIterable",
        "QByteArray",
        "QChar",
        "QItemSelection",
        "QJsonArray",
        "QJsonObject",
        "QJsonParseError",
        "QJsonValue",
        "QJsonValueRef",
        "QList",
        "QLoggingCategory",
        "QMultiHash",
        "QPointF",
        "QRegularExpressionMatch",
        "QResource",
        "QSequentialIterable",
        "QString",
      ],
    );

    config.add_cpp_ffi_generator_filter(|method| {
      if let Some(info) = &method.class_membership {
        if info.class_type.to_cpp_pseudo_code() == "QFuture<void>" {
          // template partial specialization removes these methods
          match method.name.as_ref() {
            "operator void" | "isResultReadyAt" | "result" | "resultAt" | "results" => {
              return Ok(false)
            }
            _ => {}
          }
        }
        if info.class_type.to_cpp_pseudo_code() == "QFutureIterator<void>" {
          // template partial specialization removes these methods
          match method.name.as_ref() {
            "QFutureIterator" | "operator=" => return Ok(false),
            _ => {}
          }
        }
        if info.class_type.name == "QString" {
          match method.name.as_ref() {
            "toLatin1" | "toUtf8" | "toLocal8Bit" => {
              // MacOS has non-const duplicates of these methods,
              // and that would alter Rust names of these methods
              if !info.is_const {
                return Ok(false);
              }
            }
            _ => {}
          }
        }
        if info.class_type.name == "QMetaType" {
          match method.name.as_ref() {
            "registerConverterFunction" | "unregisterConverterFunction" => {
              // only public on msvc for some technical reason
              return Ok(false);
            }
            _ => {}
          }
        }
        if info.class_type.name == "QVariant" {
          match method.name.as_ref() {
            "create" | "cmp" | "compare" | "convert" => {
              // only public on msvc for some technical reason
              return Ok(false);
            }
            _ => {}
          }
        }
      }
      let long_double = CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::LongDouble),
      };
      if &method.name == "qHash" && method.class_membership.is_none()
        && (method.arguments.len() == 1 || method.arguments.len() == 2)
        && &method.arguments[0].argument_type == &long_double
      {
        return Ok(false); // produces error on MacOS
      }
      Ok(true)
    });*/
    Ok(())
}

/// QtGui specific configuration.
fn gui_config(_config: &mut Config) -> Result<()> {
    /*
      config.add_cpp_parser_blocked_names(vec![
        "QAbstractOpenGLFunctionsPrivate",
        "QOpenGLFunctionsPrivate",
        "QOpenGLExtraFunctionsPrivate",
        "QKeySequence::isDetached",
        "QBrushData",
        "QAccessible::ActivationObserver",
        "QAccessibleImageInterface",
        "QAccessibleBridge",
        "QAccessibleBridgePlugin",
        "QAccessibleApplication",
        "QOpenGLVersionStatus",
        "QOpenGLVersionFunctionsBackend",
        "QOpenGLVersionFunctionsStorage",
        "QOpenGLTexture::TextureFormatClass",
        "QTextFrameLayoutData",
      ]);
      exclude_qvector_eq_based_methods(
        config,
        &[
          "QTextLayout::FormatRange",
          "QAbstractTextDocumentLayout::Selection",
        ],
      );
      exclude_qlist_eq_based_methods(
        config,
        &[
          "QInputMethodEvent::Attribute",
          "QTextLayout::FormatRange",
          "QTouchEvent::TouchPoint",
        ],
      );
      config.add_cpp_ffi_generator_filter(|method| {
        if let Some(info) = &method.class_membership {
          match info.class_type.to_cpp_pseudo_code().as_ref() {
            "QQueue<QInputMethodEvent::Attribute>"
            | "QQueue<QTextLayout::FormatRange>"
            | "QQueue<QTouchEvent::TouchPoint>" => match method.name.as_ref() {
              "operator==" | "operator!=" => return Ok(false),
              _ => {}
            },
            "QStack<QInputMethodEvent::Attribute>" | "QStack<QTextLayout::FormatRange>" => {
              match method.name.as_ref() {
                "operator==" | "operator!=" | "fromList" => return Ok(false),
                _ => {}
              }
            }
            "QOpenGLVersionFunctionsStorage" => match method.name.as_ref() {
              "QOpenGLVersionFunctionsStorage" | "~QOpenGLVersionFunctionsStorage" | "backend" => {
                return Ok(false)
              }
              _ => {}
            },
            _ => {}
          }
          if info.class_type.name.starts_with("QOpenGLFunctions_")
            && (info.class_type.name.ends_with("_CoreBackend")
              | info.class_type.name.ends_with("_CoreBackend::Functions")
              | info.class_type.name.ends_with("_DeprecatedBackend")
              | info
                .class_type
                .name
                .ends_with("_DeprecatedBackend::Functions"))
          {
            return Ok(false);
          }
        }
        Ok(true)
      });
    */
    Ok(())
}

/// QtWidgets specific configuration.
fn widgets_config(_config: &mut Config) -> Result<()> {
    /*
    config.add_cpp_parser_blocked_names(vec!["QWidgetData", "QWidgetItemV2"]);

    // TODO: Mac specific:
    config.add_cpp_parser_blocked_names(vec!["QMacCocoaViewContainer", "QMacNativeWidget"]);

    exclude_qlist_eq_based_methods(
      config,
      &["QTableWidgetSelectionRange", "QTextEdit::ExtraSelection"],
    );
    config.add_cpp_ffi_generator_filter(|method| {
      if let Some(info) = &method.class_membership {
        match info.class_type.to_cpp_pseudo_code().as_ref() {
          "QQueue<QTableWidgetSelectionRange>" | "QQueue<QTextEdit::ExtraSelection>" => {
            match method.name.as_ref() {
              "operator==" | "operator!=" => return Ok(false),
              _ => {}
            }
          }
          _ => {}
        }
      }
      Ok(true)
    });*/
    Ok(())
}

/// Qt3DCore specific configuration.
fn core_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DCore");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_core"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    //exclude_qvector_eq_based_methods(config, &["Qt3DCore::QNodeIdTypePair"]);
    Ok(())
}

/// Qt3DRender specific configuration.
fn render_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DRender");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_render"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    /*
    config.add_cpp_parser_blocked_names(vec![
      "Qt3DRender::QTexture1D",
      "Qt3DRender::QTexture1DArray",
      "Qt3DRender::QTexture2D",
      "Qt3DRender::QTexture2DArray",
      "Qt3DRender::QTexture3D",
      "Qt3DRender::QTextureCubeMap",
      "Qt3DRender::QTextureCubeMapArray",
      "Qt3DRender::QTexture2DMultisample",
      "Qt3DRender::QTexture2DMultisampleArray",
      "Qt3DRender::QTextureRectangle",
      "Qt3DRender::QTextureBuffer",
      "Qt3DRender::QRenderCapture",
      "Qt3DRender::QRenderCaptureReply",
      "Qt3DRender::QSortCriterion",
    ]);
    config.add_cpp_ffi_generator_filter(|method| {
      if let Some(info) = &method.class_membership {
        match info.class_type.to_cpp_pseudo_code().as_ref() {
          "Qt3DRender::QSpotLight" => match method.name.as_ref() {
            "attenuation" => return Ok(false),
            _ => {}
          },

          "Qt3DRender::QGraphicsApiFilter" => match method.name.as_ref() {
            "operator==" | "operator!=" => return Ok(false),
            _ => {}
          },

          _ => {}
        }
      }
      if method.short_text().contains("QGraphicsApiFilter") {
        println!("TEST {:?}", method);
      }
      if method.name == "Qt3DRender::operator==" || method.name == "Qt3DRender::operator!=" {
        if method.arguments.len() == 2 {
          if let CppTypeBase::Class(base) = &method.arguments[0].argument_type.base {
            if &base.name == "Qt3DRender::QGraphicsApiFilter" {
              return Ok(false);
            }
          }
        }
      }
      Ok(true)
    });*/
    Ok(())
}

/// Qt3DInput specific configuration.
fn input_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DInput");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_input"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    //config.add_cpp_parser_blocked_names(vec!["Qt3DInput::QWheelEvent"]);
    Ok(())
}

/// Qt3DLogic specific configuration.
fn logic_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DLogic");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_logic"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}

/// Qt3DExtras specific configuration.
fn extras_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DExtras");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_extras"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}

fn moqt_core_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("moqt_core"),
                prefix: None,
            }));
        }
        Ok(None)
    });

    let connect_path = CppPath::from_good_str("QObject::connect");
    let qmetamethod_ref_type =
        CppType::new_reference(true, CppType::Class(CppPath::from_good_str("QMetaMethod")));
    config.set_rust_path_hook(move |_path, name_type, data| {
        if let NameType::ApiFunction(function) = name_type {
            if let CppFfiFunctionKind::Function { cpp_function, .. } = &function.kind {
                if cpp_function.path == connect_path && cpp_function.arguments.len() >= 3 {
                    if !cpp_function.is_static_member() {
                        bail!("non-static QObject::connect is blacklisted");
                    }
                    let arg = &cpp_function.arguments[1].argument_type;
                    if arg == &qmetamethod_ref_type {
                        return Ok(Some(RustPath::from_good_str(&format!(
                            "{}::QObject::connect_by_meta_methods",
                            data.current_database.crate_name()
                        ))));
                    }
                }
            }
        }
        Ok(None)
    });

    /*config.set_movable_types_hook(|path| {
        let string = path.to_templateless_string();
        let movable = &["QMetaObject::Connection", "QPoint"];
        if movable.contains(&string.as_str()) {
            return Ok(MovableTypesHookOutput::Movable);
        }
        Ok(MovableTypesHookOutput::Unknown)
    });*/
    // TODO: blacklist QFlags<T> for FFI
    Ok(())
}

fn empty_config(_config: &mut Config) -> Result<()> {
    Ok(())
}

/// Executes the generator for a single Qt module with given configuration.
pub fn create_config(crate_name: &str) -> Result<Config> {
    info!("Preparing generator config for crate: {}", crate_name);
    let mut crate_properties = CrateProperties::new(crate_name, versions::QT_OUTPUT_CRATES_VERSION);
    let mut custom_fields = toml::value::Table::new();
    let mut package_data = toml::value::Table::new();
    package_data.insert(
        "authors".to_string(),
        toml::Value::Array(vec![toml::Value::String(
            "Pavel Strakhov <ri@idzaaus.org>".to_string(),
        )]),
    );
    let description = format!(
        "Bindings for {} C++ library (generated automatically with cpp_to_rust project)",
        lib_folder_name(crate_name)
    );
    package_data.insert("description".to_string(), toml::Value::String(description));
    let doc_url = format!("https://rust-qt.github.io/rustdoc/qt/{}", &crate_name);
    package_data.insert("documentation".to_string(), toml::Value::String(doc_url));
    package_data.insert(
        "repository".to_string(),
        toml::Value::String("https://github.com/rust-qt/cpp_to_rust".to_string()),
    );
    package_data.insert(
        "license".to_string(),
        toml::Value::String("MIT".to_string()),
    );

    custom_fields.insert("package".to_string(), toml::Value::Table(package_data));
    crate_properties.set_custom_fields(custom_fields);
    let mut config = if crate_name.starts_with("moqt_") {
        let mut config = Config::new(crate_properties);
        let moqt_path = PathBuf::from(
            ::std::env::var("MOQT_INSTALL_DIR")
                .with_context(|_| "MOQT_INSTALL_DIR env var is missing")?,
        );

        config.add_include_directive(format!("{}.h", crate_name));
        let include_path = moqt_path.join("include");
        if !include_path.exists() {
            bail!("Path does not exist: {}", include_path.display());
        }
        let lib_path = moqt_path.join("lib");
        if !lib_path.exists() {
            bail!("Path does not exist: {}", lib_path.display());
        }
        let sublib_include_path = include_path.join(crate_name);
        if !sublib_include_path.exists() {
            bail!("Path does not exist: {}", sublib_include_path.display());
        }
        {
            let mut paths = CppBuildPaths::new();
            paths.add_include_path(&sublib_include_path);
            paths.add_lib_path(&lib_path);

            for &lib in lib_dependencies(crate_name)? {
                let dep_include_path = include_path.join(lib);
                if !dep_include_path.exists() {
                    bail!("Path does not exist: {}", dep_include_path.display());
                }
                paths.add_include_path(&dep_include_path);
            }
            config.set_cpp_build_paths(paths);
        }
        config.add_target_include_path(&sublib_include_path);

        {
            let mut data = CppBuildConfigData::new();
            data.add_linked_lib(crate_name);
            for &lib in lib_dependencies(crate_name)? {
                data.add_linked_lib(lib);
            }
            data.set_library_type(CppLibraryType::Shared);
            config
                .cpp_build_config_mut()
                .add(target::Condition::True, data);
        }
        {
            let mut data = CppBuildConfigData::new();
            data.add_compiler_flag("-fPIC");
            data.add_compiler_flag("-std=gnu++11");
            config
                .cpp_build_config_mut()
                .add(target::Condition::Env(target::Env::Msvc).negate(), data);
        }
        if target::current_env() == target::Env::Msvc {
            config.add_cpp_parser_argument("-std=c++14");
        } else {
            config.add_cpp_parser_argument("-std=gnu++11");
        }
        //    let cpp_config_data = CppBuildConfigData {
        //      linked_libs: vec![crate_name.to_string()],
        //      linked_frameworks: Vec::new(),
        //
        //    }
        //...
        config
    } else {
        crate_properties.remove_default_build_dependencies();
        crate_properties.add_build_dependency(
            "qt_ritual_build",
            versions::QT_RITUAL_BUILD_VERSION,
            Some(repo_dir_path("qt_ritual_build")?),
        );

        let mut config = Config::new(crate_properties);

        let qt_config = get_full_build_config(crate_name)?;
        config.set_cpp_build_config(qt_config.cpp_build_config);
        config.set_cpp_build_paths(qt_config.cpp_build_paths);

        config.add_target_include_path(&qt_config.installation_data.lib_include_path);
        config.set_cpp_lib_version(qt_config.installation_data.qt_version.as_str());
        // TODO: does parsing work on MacOS without adding "-F"?

        config.add_include_directive(&lib_folder_name(crate_name));

        // DEBUG!
        //config.add_include_directive("QObject");
        //config.add_include_directive("QMetaObject");

        // TODO: allow to override parser flags
        config.add_cpp_parser_arguments(vec!["-fPIC", "-fcxx-exceptions"]);

        if target::current_env() == target::Env::Msvc {
            config.add_cpp_parser_argument("-std=c++14");
        } else {
            config.add_cpp_parser_argument("-std=gnu++11");
        }
        //config.add_cpp_parser_blocked_name(CppName::from_one_part("qt_check_for_QGADGET_macro"));

        let lib_include_path = qt_config.installation_data.lib_include_path.clone();

        let steps = config.processing_steps_mut();
        steps.add_after(&["cpp_parser"], "qt_fix_header_names", move |data| {
            fix_header_names(data.current_database.cpp_items_mut(), &lib_include_path)
        })?;

        let crate_name_clone = crate_name.to_string();
        let docs_path = qt_config.installation_data.docs_path.clone();

        steps.add_after(&["cpp_parser"], "qt_doc_parser", move |data| {
            parse_docs(data, &crate_name_clone, &docs_path)
        })?;

        config
    };

    let steps = config.processing_steps_mut();
    steps.add_after(
        &["cpp_parser"],
        "qt_detect_signals_and_slots",
        detect_signals_and_slots,
    )?;

    steps.add_after(
        &["qt_detect_signals_and_slots"],
        "add_slot_wrappers",
        add_slot_wrappers,
    )?;

    config.set_crate_template_path(repo_dir_path("qt_ritual/crate_templates")?.join(&crate_name));

    let lib_config = match crate_name {
        "qt_core" => core_config,
        "qt_gui" => gui_config,
        "qt_widgets" => widgets_config,
        "qt_3d_core" => core_3d_config,
        "qt_3d_render" => render_3d_config,
        "qt_3d_input" => input_3d_config,
        "qt_3d_logic" => logic_3d_config,
        "qt_3d_extras" => extras_3d_config,
        "qt_ui_tools" => empty_config,
        "moqt_core" => moqt_core_config,
        "moqt_gui" => empty_config,
        _ => bail!("Unknown crate name: {}", crate_name),
    };
    lib_config(&mut config)?;

    config.set_dependent_cpp_crates(
        lib_dependencies(crate_name)?
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    Ok(config)
}

pub fn global_config() -> GlobalConfig {
    let mut config = GlobalConfig::new();
    config.set_all_crate_names(all_crate_names().iter().map(|s| s.to_string()).collect());
    config.set_create_config_hook(create_config);
    config
}
