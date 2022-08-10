; ModuleID = 'probe5.440c9001-cgu.0'
source_filename = "probe5.440c9001-cgu.0"
target datalayout = "e-m:o-i64:64-i128:128-n32:64-S128"
target triple = "arm64-apple-macosx11.0.0"

%"core::panic::location::Location" = type { { [0 x i8]*, i64 }, i32, i32 }

@alloc5 = private unnamed_addr constant <{ [77 x i8] }> <{ [77 x i8] c"/rustc/affe0d3a00e92fa7885e3f5d2c5073fde432d154/library/core/src/ops/arith.rs" }>, align 1
@alloc6 = private unnamed_addr constant <{ i8*, [16 x i8] }> <{ i8* getelementptr inbounds (<{ [77 x i8] }>, <{ [77 x i8] }>* @alloc5, i32 0, i32 0, i32 0), [16 x i8] c"M\00\00\00\00\00\00\00\12\03\00\00\01\00\00\00" }>, align 8
@str.0 = internal constant [28 x i8] c"attempt to add with overflow"
@alloc3 = private unnamed_addr constant <{ [4 x i8] }> <{ [4 x i8] c"\02\00\00\00" }>, align 4

; <i32 as core::ops::arith::AddAssign<&i32>>::add_assign
; Function Attrs: inlinehint uwtable
define internal void @"_ZN66_$LT$i32$u20$as$u20$core..ops..arith..AddAssign$LT$$RF$i32$GT$$GT$10add_assign17hbee54b1b6192b767E"(i32* align 4 %self, i32* align 4 %other) unnamed_addr #0 {
start:
  %other1 = load i32, i32* %other, align 4
  %0 = load i32, i32* %self, align 4
  %1 = call { i32, i1 } @llvm.sadd.with.overflow.i32(i32 %0, i32 %other1)
  %_6.0 = extractvalue { i32, i1 } %1, 0
  %_6.1 = extractvalue { i32, i1 } %1, 1
  %2 = call i1 @llvm.expect.i1(i1 %_6.1, i1 false)
  br i1 %2, label %panic, label %bb1

bb1:                                              ; preds = %start
  store i32 %_6.0, i32* %self, align 4
  ret void

panic:                                            ; preds = %start
; call core::panicking::panic
  call void @_ZN4core9panicking5panic17h07a28ef61f653d1dE([0 x i8]* align 1 bitcast ([28 x i8]* @str.0 to [0 x i8]*), i64 28, %"core::panic::location::Location"* align 8 bitcast (<{ i8*, [16 x i8] }>* @alloc6 to %"core::panic::location::Location"*)) #5
  unreachable
}

; probe5::probe
; Function Attrs: uwtable
define void @_ZN6probe55probe17h895a6ec875d1cb75E() unnamed_addr #1 {
start:
  %x = alloca i32, align 4
  store i32 1, i32* %x, align 4
; call <i32 as core::ops::arith::AddAssign<&i32>>::add_assign
  call void @"_ZN66_$LT$i32$u20$as$u20$core..ops..arith..AddAssign$LT$$RF$i32$GT$$GT$10add_assign17hbee54b1b6192b767E"(i32* align 4 %x, i32* align 4 bitcast (<{ [4 x i8] }>* @alloc3 to i32*))
  br label %bb1

bb1:                                              ; preds = %start
  ret void
}

; Function Attrs: nofree nosync nounwind readnone speculatable willreturn
declare { i32, i1 } @llvm.sadd.with.overflow.i32(i32, i32) #2

; Function Attrs: nofree nosync nounwind readnone willreturn
declare i1 @llvm.expect.i1(i1, i1) #3

; core::panicking::panic
; Function Attrs: cold noinline noreturn uwtable
declare void @_ZN4core9panicking5panic17h07a28ef61f653d1dE([0 x i8]* align 1, i64, %"core::panic::location::Location"* align 8) unnamed_addr #4

attributes #0 = { inlinehint uwtable "frame-pointer"="non-leaf" "target-cpu"="apple-a14" }
attributes #1 = { uwtable "frame-pointer"="non-leaf" "target-cpu"="apple-a14" }
attributes #2 = { nofree nosync nounwind readnone speculatable willreturn }
attributes #3 = { nofree nosync nounwind readnone willreturn }
attributes #4 = { cold noinline noreturn uwtable "frame-pointer"="non-leaf" "target-cpu"="apple-a14" }
attributes #5 = { noreturn }

!llvm.module.flags = !{!0}

!0 = !{i32 7, !"PIC Level", i32 2}
