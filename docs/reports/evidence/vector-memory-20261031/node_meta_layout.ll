; ModuleID = 'node_meta_layout.c35b31fef8d23f39-cgu.0'
source_filename = "node_meta_layout.c35b31fef8d23f39-cgu.0"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-msvc"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define noundef i64 @node_metadata_size() unnamed_addr #0 personality ptr @__CxxFrameHandler3 {
start:
  ret i64 96
}

declare i32 @__CxxFrameHandler3(...) unnamed_addr #1

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable "target-cpu"="x86-64" "target-features"="+cx16,+sse,+sse2,+sse3,+sahf" }
attributes #1 = { "target-cpu"="x86-64" }

!llvm.module.flags = !{!0}
!llvm.ident = !{!1}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{!"rustc version 1.96.0 (ac68faa20 2026-05-25)"}
