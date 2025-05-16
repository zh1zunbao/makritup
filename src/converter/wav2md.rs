use hound::WavReader;
use std::io::Cursor;
use vosk::{Model, Recognizer};

// Helper function to read wave data from a byte stream
fn retrieve_wave_samples(stream: &[u8]) -> Result<(Vec<i16>, u32), String> {
    let cursor = Cursor::new(stream);
    // map_err:
    //   作用: 用于转换 Result 类型中的 Err 值。
    //         如果 Result 是 Ok(T)，它保持不变。
    //         如果 Result 是 Err(E)，它会调用一个闭包，并将 E 作为参数传递给闭包，闭包的返回值将成为新的 Err 值。
    //   用法: result_expression.map_err(|original_error| new_error_value)
    //         在这里，如果 WavReader::new(cursor) 返回 Err(e)，则将错误 e 转换为一个格式化的字符串。
    let reader = WavReader::new(cursor).map_err(|e| format!("Failed to read WAV stream: {}", e))?;

    let spec = reader.spec();
    if spec.channels != 1 {
        return Err(format!("Mono audio required (channels: {})", spec.channels));
    }
    if spec.bits_per_sample != 16 {
        return Err(format!("16-bit depth required (depth: {})", spec.bits_per_sample));
    }
    // Sample rate will be checked in the main run function if necessary.

    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        // collect::<Result<Vec<_>, _>>() 可能返回一个包含原始错误类型的 Result。
        // map_err 用于将这个原始错误类型转换为我们期望的 String 错误类型。
        //   作用: 转换 Result 中的 Err 部分。如果 Result 是 Ok，则什么也不做。
        //         如果 Result 是 Err(original_error)，则调用闭包 f(original_error)，
        //         闭包的返回值将作为新的 Err 值。
        //   用法: some_result.map_err(|err_val| format!("New error: {}", err_val))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read samples: {}", e))?;

    Ok((samples, spec.sample_rate))
}

pub fn run(file_stream: &[u8]) -> Result<String, String> {
    let model_path = "/tmp/models/vosk-model-small-en-us-0.15"; // Assuming this path is accessible

    // ok_or_else:
    //   作用: 用于将 Option<T> 类型转换为 Result<T, E> 类型。
    //         如果 Option 是 Some(v)，它会返回 Ok(v)。
    //         如果 Option 是 None，它会调用一个闭包，闭包的返回值将作为 Err(E) 中的 E 值。
    //         这允许你懒惰地计算错误值，仅在 Option 为 None 时才执行闭包。
    //   用法: option_expression.ok_or_else(|| error_value_if_none)
    //         在这里，如果 Model::new(model_path) 返回 None (表示模型加载失败),
    //         则执行闭包 || format!("Failed to load model: {}", model_path)，
    //         其结果（一个String）将作为 Err 返回。
    let model = Model::new(model_path)
        .ok_or_else(|| format!("Failed to load model: {}", model_path))?;

    let (samples, sample_rate) = retrieve_wave_samples(file_stream)
        .map_err(|e| format!("Failed to read audio stream: {}", e))?;

    // if sample_rate != 16000 {
    //     return Err(format!(
    //         "16000Hz sample rate required, current is {}Hz",
    //         sample_rate
    //     ));
    // }

    let mut recognizer = Recognizer::new(&model, sample_rate as f32)
        .ok_or_else(|| "Recognizer initialization failed".to_string())?;

    recognizer.accept_waveform(&samples)
        .map_err(|e| format!("Failed to process audio stream: {}", e))?;
        
    let result = recognizer.final_result();
    let text = result
        .single()
        .map(|alt| alt.text)
        .unwrap_or("[No valid content recognized]");

    Ok(format!(
        "# Audio Transcription\n\n\
        ## Basic Information\n\
        - **Sample Rate**: {} Hz\n\
        - **Recognition Engine**: Vosk (Model: {})\n\n\
        ## Transcription\n{}",
        sample_rate,
        model_path, // Using model_path to indicate which model was used
        text
    ))
}
