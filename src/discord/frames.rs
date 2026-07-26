use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("Failed to read from stream: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn read_frame<R>(reader: &mut R) -> Result<(u32, serde_json::Value), ReadError>
where
    R: AsyncReadExt + Unpin,
{
    let mut header_buf = [0; 8];
    reader.read_exact(&mut header_buf).await?;

    let opcode = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
    let length = u32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;

    let mut payload_buf = vec![0; length];
    reader.read_exact(&mut payload_buf).await?;

    let payload_str = std::str::from_utf8(&payload_buf)?;
    let payload_json = serde_json::from_str(payload_str)?;
    Ok((opcode, payload_json))
}

#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("Failed to write to stream: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn write_frame<W>(
    writer: &mut W,
    opcode: u32,
    payload: impl Into<serde_json::Value>,
) -> Result<(), WriteError>
where
    W: AsyncWriteExt + Unpin,
{
    let payload_str = serde_json::to_string(&payload.into())?;
    let len = payload_str.len() as u32;

    writer.write_all(&opcode.to_le_bytes()).await?;
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(payload_str.as_bytes()).await?;

    Ok(())
}
