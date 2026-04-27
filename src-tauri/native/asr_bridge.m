#import <Foundation/Foundation.h>
#import <Speech/Speech.h>
#import <AVFoundation/AVFoundation.h>

static char *dup_cstr(NSString *s) {
    const char *utf8 = [s UTF8String];
    if (utf8 == NULL) {
        return strdup("");
    }
    return strdup(utf8);
}

static BOOL spin_until(BOOL (^condition)(void), NSTimeInterval timeout) {
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:timeout];
    while (!condition()) {
        if ([deadline timeIntervalSinceNow] <= 0) {
            return NO;
        }
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                 beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
    }
    return YES;
}

static NSString *format_error(NSError *error) {
    NSMutableArray<NSString *> *parts = [NSMutableArray array];
    [parts addObject:[NSString stringWithFormat:@"domain=%@", error.domain ?: @""]];
    [parts addObject:[NSString stringWithFormat:@"code=%ld", (long)error.code]];
    [parts addObject:[NSString stringWithFormat:@"desc=%@", error.localizedDescription ?: @""]];
    if (error.localizedFailureReason.length > 0) {
        [parts addObject:[NSString stringWithFormat:@"reason=%@", error.localizedFailureReason]];
    }
    if (error.localizedRecoverySuggestion.length > 0) {
        [parts addObject:[NSString stringWithFormat:@"suggestion=%@", error.localizedRecoverySuggestion]];
    }
    return [parts componentsJoinedByString:@" | "];
}

static void run_recognition(NSURL *url,
                            NSLocale *locale,
                            NSTimeInterval timeoutSeconds,
                            NSDate *start,
                            NSString **outText,
                            NSString **outErr) {
    SFSpeechRecognizer *recognizer = nil;
    if (locale != nil) {
        recognizer = [[SFSpeechRecognizer alloc] initWithLocale:locale];
    } else {
        recognizer = [[SFSpeechRecognizer alloc] init];
    }

    if (recognizer == nil) {
        *outText = @"";
        *outErr = @"speech recognizer unavailable";
        return;
    }

    if (!recognizer.isAvailable) {
        *outText = @"";
        *outErr = @"speech recognizer unavailable isAvailable=false";
        return;
    }

    SFSpeechURLRecognitionRequest *request =
        [[SFSpeechURLRecognitionRequest alloc] initWithURL:url];
    request.shouldReportPartialResults = NO;

    __block NSString *finalText = @"";
    __block NSString *finalErr = @"";
    __block BOOL didFinish = NO;

    __block SFSpeechRecognitionTask *task =
        [recognizer recognitionTaskWithRequest:request
                                 resultHandler:^(SFSpeechRecognitionResult * _Nullable result,
                                                 NSError * _Nullable error) {
        if (error != nil) {
            finalErr = format_error(error);
            didFinish = YES;
            return;
        }

        if (result != nil && result.isFinal) {
            finalText = result.bestTranscription.formattedString ?: @"";
            didFinish = YES;
        }
    }];

    NSTimeInterval elapsed = [[NSDate date] timeIntervalSinceDate:start];
    NSTimeInterval remaining = MAX(1.0, timeoutSeconds - elapsed);
    if (!spin_until(^BOOL {
        return didFinish;
    }, remaining)) {
        [task cancel];
        *outText = @"";
        *outErr = @"speech recognition timeout";
        return;
    }

    *outText = finalText ?: @"";
    *outErr = finalErr.length > 0 ? finalErr : nil;
}

char *asr_request_permissions(void) {
    @autoreleasepool {
        NSTimeInterval timeoutSeconds = 15;

        __block BOOL micResolved = NO;
        __block BOOL micGranted = NO;
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio
                                 completionHandler:^(BOOL granted) {
            micGranted = granted;
            micResolved = YES;
        }];

        __block BOOL speechResolved = NO;
        __block SFSpeechRecognizerAuthorizationStatus speechStatus = SFSpeechRecognizerAuthorizationStatusNotDetermined;
        [SFSpeechRecognizer requestAuthorization:^(SFSpeechRecognizerAuthorizationStatus status) {
            speechStatus = status;
            speechResolved = YES;
        }];

        if (!spin_until(^BOOL {
            return micResolved && speechResolved;
        }, timeoutSeconds)) {
            return dup_cstr(@"permissions request timeout");
        }

        if (!micGranted) {
            return dup_cstr(@"microphone authorization denied");
        }

        if (speechStatus != SFSpeechRecognizerAuthorizationStatusAuthorized) {
            return dup_cstr([NSString stringWithFormat:@"speech authorization denied status=%ld", (long)speechStatus]);
        }

        return NULL;
    }
}

char *asr_transcribe_wav(const char *wav_path) {
    @autoreleasepool {
        if (wav_path == NULL) {
            return dup_cstr(@"ERR:missing wav path");
        }

        NSString *wavPath = [NSString stringWithUTF8String:wav_path];
        if (wavPath.length == 0) {
            return dup_cstr(@"ERR:missing wav path");
        }

        NSURL *url = [NSURL fileURLWithPath:wavPath];
        NSTimeInterval timeoutSeconds = 12;
        NSDate *start = [NSDate date];

        __block BOOL authResolved = NO;
        __block SFSpeechRecognizerAuthorizationStatus authStatus = SFSpeechRecognizerAuthorizationStatusNotDetermined;
        [SFSpeechRecognizer requestAuthorization:^(SFSpeechRecognizerAuthorizationStatus status) {
            authStatus = status;
            authResolved = YES;
        }];

        if (!spin_until(^BOOL {
            return authResolved;
        }, timeoutSeconds)) {
            return dup_cstr(@"ERR:speech authorization timeout");
        }

        if (authStatus != SFSpeechRecognizerAuthorizationStatusAuthorized) {
            return dup_cstr([NSString stringWithFormat:@"ERR:speech authorization denied status=%ld", (long)authStatus]);
        }

        NSString *outputText = @"";
        NSString *errorText = nil;
        run_recognition(url, [[NSLocale alloc] initWithLocaleIdentifier:@"zh-CN"], timeoutSeconds, start, &outputText, &errorText);

        if (errorText != nil && ([errorText containsString:@"code=-11800"] || [errorText containsString:@"(-17913)"])) {
            NSString *fallbackText = @"";
            NSString *fallbackErr = nil;
            run_recognition(url, nil, timeoutSeconds, start, &fallbackText, &fallbackErr);
            outputText = fallbackText;
            errorText = fallbackErr;
        }

        if (errorText.length > 0) {
            return dup_cstr([@"ERR:" stringByAppendingString:errorText]);
        }

        return dup_cstr([@"OK:" stringByAppendingString:(outputText ?: @"")]);
    }
}

void asr_string_free(char *ptr) {
    if (ptr != NULL) {
        free(ptr);
    }
}
