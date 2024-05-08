use std::io::{self, BufRead, BufReader, Write};
use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream, ToNsName};
use serde::{Deserialize, Serialize};
use checksum_dir::checksum;

#[derive(Debug)]
pub enum SlibError {
    InvalidCommand(u8),
    InvalidServerHash(Vec<u8>),
}

pub const NAME: &str = "slib.socket";

pub const HASH: [u8; 32] = checksum!("./src");

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum  Commands {
    /// Verify slib version
    Verify,
    /// Shutdown the server
    Shutdown,
    /// Fetch IDs of remote songs and playlists
    Fetch,
    /// Tell the Subsonic server to rescan
    Scan,
    /// Get the status of playback
    Status,

    /// Restart currently playing song
    Restart,
    /// Play (unpause) Playback
    Play,
    /// Stop and clear queue
    Stop,
    /// Pause Playback
    Pause,
    /// Skip the currentlly playing song
    Skip,

    /// Add a song to the queue
    QueueAdd{id: Item, position: u8},
    /// Remove a song from the queue
    QueueRemove(Item),

    /// Adjust volume by percent
    VolumeAdjust(u8),
    /// Set the volume by percent
    VolumeSet(u8),

    /// Search for a query
    Search(String),
    /// Download a song for offline playback
    Download(Item),
    /// Delete a song from offline playback
    Delete(Item),
    /// Favorite a song on the Subsonic server
    Star(Item),

    /// Download all the songs from a playlist
    PlaylistDownload(Item),
    /// Upload changes on a local playlist
    PlaylistUpload(Item),
    /// Create a new local playlist
    PlaylistNew{name: String},
    /// Add to a local playlist
    PlaylistAddTo{playlist: Item, id: Item},
    /// Remove from a local playlist
    PlaylistRemoveFrom{playlist: Item, id: Item},
    /// Delete a local playlist
    PlaylistDelete(Item),

    /// Get the info of a song
    SongInfo(Item),
    /// Get the info of a album
    AlbumInfo(Item),
}


pub trait Daemon {
    fn shutdown(&self)                                              -> bool;
    fn fetch(&self)                                                 -> Vec<Item>;
    fn scan(&self)                                                  -> bool;
    fn status(&self)                                                -> Status;
    fn restart(&self)                                               -> bool;
    fn play(&self)                                                  -> bool;
    fn stop(&self)                                                  -> bool;
    fn pause(&self)                                                 -> bool;
    fn skip(&self)                                                  -> bool;
    fn queue_add(&self, id: Item, position: u8)                     -> bool;
    fn queue_remove(&self, id: Item)                                -> bool;
    fn volume_adjust(&self, amount: u8)                             -> bool;
    fn volume_set(&self, amount: u8)                                -> bool;
    fn search(&self, query: String)                                 -> Vec<Item>;
    fn download(&self, id: Item)                                    -> bool;
    fn delete(&self, id: Item)                                      -> bool;
    fn star(&self, id: Item)                                        -> bool; 
    fn playlist_download(&self, id: Item)                           -> bool;
    fn playlist_upload(&self, id: Item)                             -> bool;
    fn playlist_new(&self, name: String)                            -> bool;
    fn playlist_add_to(&self, playlist: Item, id: Item)             -> bool;
    fn playlist_remove_from(&self, playlist: Item, id: Item)        -> bool;
    fn playlist_delete(&self, id: Item)                             -> bool;
    fn song_info(&self, id: Item)                                   -> SongInfo;
    fn album_info(&self, id: Item)                                  -> AlbumInfo;


    fn start(&self) 
    { 
        //  Try to put the name in the Namespace
        let name = NAME.to_ns_name::<GenericNamespaced>().unwrap();

        // Create our local socket listener using the name
        let opts = ListenerOptions::new().name(name);
        let listener = opts.create_sync().unwrap();

        // Create a buffer we can write our input into. The size may need to be changed
        let mut buffer = String::with_capacity(128);

        // Infinite Iterator over the connections incoming in from the listener
        'listen: for conn in listener.incoming().filter_map(handle_error)
        {
            // Make a reader for the connection
            let mut conn = BufReader::new(conn);
            // Read from the connection
            let _ = conn.read_line(&mut buffer);

            // Turn the into an enum from a json string
            let command = serde_json::from_str::<Commands>(&buffer).unwrap();
            // Get the response from the Daemon
            let response = self.interpert_command(command);

            // Send the response back
            conn.get_mut().write_all(serde_json::to_string(&response).unwrap().as_bytes()).expect("failed to send");
            conn.get_mut().write_all(b"\n").expect("failed to send");

            // Turn the command back into an enum again
            let command = serde_json::from_str::<Commands>(&buffer).unwrap();
            // Clean up
            buffer.clear();

            let t = true;
            // If it is told to shutdown
            if  command == Commands::Shutdown
                &&
            // and the daemon is good to stop
                response == serde_json::to_string(&t).unwrap()
            {
                break 'listen;
            }
        }

    }


    fn interpert_command(&self, c: Commands) -> String {
        match c {
                Commands::Verify                           => { println!("Verifying... "); serde_json::to_string( &HASH.to_vec()                           ) },
                Commands::Shutdown                         => { serde_json::to_string( &self.shutdown()                         ) },
                Commands::Fetch                            => { serde_json::to_string( &self.fetch()                            ) },
                Commands::Scan                             => { serde_json::to_string( &self.scan()                             ) },
                Commands::Status                           => { serde_json::to_string( &self.status()                           ) },
                Commands::Restart                          => { serde_json::to_string( &self.restart()                          ) },
                Commands::Play                             => { serde_json::to_string( &self.play()                             ) },
                Commands::Stop                             => { serde_json::to_string( &self.stop()                             ) },
                Commands::Pause                            => { serde_json::to_string( &self.pause()                            ) },
                Commands::Skip                             => { serde_json::to_string( &self.skip()                             ) },
                Commands::QueueAdd{id, position}           => { serde_json::to_string( &self.queue_add(id, position)            ) },
                Commands::QueueRemove(id)                  => { serde_json::to_string( &self.queue_remove(id)                   ) },
                Commands::VolumeAdjust(amount)             => { serde_json::to_string( &self.volume_adjust(amount)              ) },
                Commands::VolumeSet(amount)                => { serde_json::to_string( &self.volume_set(amount)                 ) },
                Commands::Search(query)                    => { serde_json::to_string( &self.search(query)                      ) },
                Commands::Download(id)                     => { serde_json::to_string( &self.download(id)                       ) },
                Commands::Delete(id)                       => { serde_json::to_string( &self.delete(id)                         ) },
                Commands::Star(id)                         => { serde_json::to_string( &self.star(id)                           ) },
                Commands::PlaylistDownload(id)             => { serde_json::to_string( &self.playlist_download(id)              ) },
                Commands::PlaylistUpload(id)               => { serde_json::to_string( &self.playlist_upload(id)                ) },
                Commands::PlaylistNew{name}                => { serde_json::to_string( &self.playlist_new(name)                 ) },
                Commands::PlaylistAddTo{playlist, id}      => { serde_json::to_string( &self.playlist_add_to(playlist, id)      ) },
                Commands::PlaylistRemoveFrom{playlist, id} => { serde_json::to_string( &self.playlist_remove_from(playlist, id) ) },
                Commands::PlaylistDelete(id)               => { serde_json::to_string( &self.playlist_delete(id)                ) },
                Commands::SongInfo(id)                     => { serde_json::to_string( &self.song_info(id)                      ) },
                Commands::AlbumInfo(id)                    => { serde_json::to_string( &self.album_info(id)                     ) },
            }.unwrap()
    }
}

fn handle_error(conn: io::Result<Stream>) -> Option<Stream> {
    match conn {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("Incomming connection failed: {e}");
            None
        }
    }
}

pub struct Client;
impl Client {
    pub fn new() -> Result<Client,SlibError> 
    {
        Ok(Client{})
    }

    pub fn send_command(&self, c: Commands) -> String {
        let mut buffer = String::with_capacity(128);
        let conn = Stream::connect(NAME.to_ns_name::<GenericNamespaced>().unwrap()).unwrap();
        let mut conn = BufReader::new(conn);
        let _ = conn.get_mut().write_all(serde_json::to_string(&c).unwrap().as_bytes());
        let _ = conn.get_mut().write_all(b"\n");
        let _ = conn.read_line(&mut buffer);
        buffer
    }
}

#[derive(Deserialize,Serialize)]
pub struct Status {
    playing: bool,
    current_song: Item,
    queue: Vec<String>,
}

#[derive(Deserialize,Serialize, Debug, PartialEq, Eq)]
pub struct Item {
    name: String,
    id: String,
    image_path: String,
}

#[derive(Deserialize,Serialize)]
pub struct SongInfo {
    length: f32,
    album: Item,
    artist: String,
}

#[derive(Deserialize,Serialize)]
pub struct AlbumInfo {
    songs: Vec<Item>,
    artist: String,
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    struct Server;
    impl Daemon for Server 
    {
        fn shutdown(&self) -> bool {
            true
        }

        fn fetch(&self)                                                 -> Vec<Item> {
            todo!()
        }

        fn scan(&self)                                                  -> bool {
            todo!()
        }

        fn status(&self)                                                -> Status {
            todo!()
        }

        fn restart(&self)                                               -> bool {
            todo!()
        }

        fn play(&self)                                                  -> bool {
            todo!()
        }

        fn stop(&self)                                                  -> bool {
            todo!()
        }

        fn pause(&self)                                                 -> bool {
            todo!()
        }

        fn skip(&self)                                                  -> bool {
            todo!()
        }

        fn queue_add(&self, id: Item, position: u8)                     -> bool {
            todo!()
        }

        fn queue_remove(&self, id: Item)                                -> bool {
            todo!()
        }

        fn volume_adjust(&self, amount: u8)                             -> bool {
            todo!()
        }

        fn volume_set(&self, amount: u8)                                -> bool {
            todo!()
        }

        fn search(&self, query: String)                                 -> Vec<Item> {
            todo!()
        }

        fn download(&self, id: Item)                                    -> bool {
            todo!()
        }

        fn delete(&self, id: Item)                                      -> bool {
            todo!()
        }

        fn star(&self, id: Item)                                        -> bool {
            todo!()
        }

        fn playlist_download(&self, id: Item)                           -> bool {
            todo!()
        }

        fn playlist_upload(&self, id: Item)                             -> bool {
            todo!()
        }

        fn playlist_new(&self, name: String)                            -> bool {
            todo!()
        }

        fn playlist_add_to(&self, playlist: Item, id: Item)             -> bool {
            todo!()
        }

        fn playlist_remove_from(&self, playlist: Item, id: Item)        -> bool {
            todo!()
        }

        fn playlist_delete(&self, id: Item)                             -> bool {
            todo!()
        }

        fn song_info(&self, id: Item)                                   -> SongInfo {
            todo!()
        }

        fn album_info(&self, id: Item)                                  -> AlbumInfo {
            todo!()
        }
    }

    #[test]
    fn verify() 
    {
        thread::spawn ( move || {
            let test_server = Server{};
            test_server.start();
        });

        thread::sleep(Duration::from_secs(1));
        let client = Client::new().unwrap();
        client.send_command(Commands::Shutdown);
    }

}
