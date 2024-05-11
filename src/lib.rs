use std::{io::{self, BufRead, BufReader, Write}, time::Duration}; 
use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream, ToNsName};
use serde::{Deserialize, Serialize};
use checksum_dir::checksum;

#[derive(Debug)]
pub enum SlibError {
    InvalidCommand(u8),
    InvalidServerHash(Vec<u8>),
}

const NAME: &str = "slib.socket";

const HASH: [u8; 32] = checksum!("./src");

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum  Commands {
    /// Verify slib version
    Verify,
    /// Shutdown the server
    Shutdown,
    
    /// Return all artists
    FetchArtists,
    /// Return all albums
    FetchAlbums,
    /// Return all playlists
    FetchPlaylists,
    /// Return all songs
    FetchSongs,

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
    /// Return all artists
    fn fetch_artists(&self)                                         -> Vec<Item>;
    /// Return all albums
    fn fetch_albums(&self)                                          -> Vec<Item>;
    /// Return all songs
    fn fetch_playlists(&self)                                       -> Vec<Item>;
    /// Return all songs
    fn fetch_songs(&self)                                           -> Vec<Item>;
    /// Tell the Subsonic server to rescan
    fn scan(&self)                                                  -> bool;
    /// Get the status of playback
    fn status(&self)                                                -> &Status;
    /// Restart currently playing song
    fn restart(&self)                                               -> bool;
    /// Play (unpause) Playback
    fn play(&self)                                                  -> bool;
    /// Stop and clear queue
    fn stop(&self)                                                  -> bool;
    /// Pause Playback
    fn pause(&self)                                                 -> bool;
    /// Skip the currentlly playing song
    fn skip(&self)                                                  -> bool;
    /// Add a song to the queue
    fn queue_add(&self, id: Item, position: u8)                     -> bool;
    /// Remove a song from the queue
    fn queue_remove(&self, id: Item)                                -> bool;
    /// Adjust volume by percent
    fn volume_adjust(&self, amount: u8)                             -> bool;
    /// Set the volume by percent
    fn volume_set(&self, amount: u8)                                -> bool;
    /// Search for a query
    fn search(&self, query: String)                                 -> Vec<Item>;
    /// Download a song for offline playback
    fn download(&self, id: Item)                                    -> bool;
    /// Delete a song from offline playback
    fn delete(&self, id: Item)                                      -> bool;
    /// Favorite a song on the Subsonic server
    fn star(&self, id: Item)                                        -> bool; 
    /// Download all the songs from a playlist
    fn playlist_download(&self, id: Item)                           -> bool;
    /// Upload changes on a local playlist
    fn playlist_upload(&self, id: Item)                             -> bool;
    /// Create a new local playlist
    fn playlist_new(&self, name: String)                            -> bool;
    /// Add to a local playlist
    fn playlist_add_to(&self, playlist: Item, id: Item)             -> bool;
    /// Remove from a local playlist
    fn playlist_remove_from(&self, playlist: Item, id: Item)        -> bool;
    /// Delete a local playlist
    fn playlist_delete(&self, id: Item)                             -> bool;
    /// Get the info of a song
    fn song_info(&self, id: Item)                                   -> Option<SongInfo>;
    /// Get the info of a album
    fn album_info(&self, id: Item)                                  -> Option<AlbumInfo>;


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

            // Remove newline from the end
            buffer.pop();

            // Turn the into an enum from a json string
            let command = serde_json::from_str::<Commands>(&buffer).unwrap();


            // Get the response from the Daemon
            let response = self.interpert_command(command);

            // Send the response back
            conn.get_mut().write_all(&response.as_bytes()).expect("failed to send");
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
                Commands::Verify                           => { serde_json::to_string( &HASH.to_vec()                           ) },
                Commands::Shutdown                         => { serde_json::to_string( &self.shutdown()                         ) },
                Commands::FetchArtists                     => { serde_json::to_string( &self.fetch_artists()                    ) },
                Commands::FetchAlbums                      => { serde_json::to_string( &self.fetch_albums()                     ) },
                Commands::FetchPlaylists                   => { serde_json::to_string( &self.fetch_playlists()                  ) },
                Commands::FetchSongs                       => { serde_json::to_string( &self.fetch_songs()                      ) },
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
        let command = Commands::Verify;

        let mut buffer = String::with_capacity(128);
        let conn = Stream::connect(NAME.to_ns_name::<GenericNamespaced>().unwrap()).unwrap();
        let mut conn = BufReader::new(conn);
        let _ = conn.get_mut().write_all(serde_json::to_string(&command).unwrap().as_bytes());
        let _ = conn.get_mut().write_all(b"\n");
        let _ = conn.read_line(&mut buffer);

        // Remove newline from the end
        let mut buffer = buffer.chars();
        buffer.next_back();
        let buffer = buffer.as_str();

        let hash = serde_json::from_str::<Vec<u8>>(&buffer).unwrap();
        let matching = hash.iter().zip(HASH.to_vec().iter()).filter(|&(a, b)| a == b).count();
        if matching == hash.len() 
        {
            Ok(Client{})
        }
        else
        {
            Err(SlibError::InvalidServerHash(hash))
        }
        
    }

    fn send_command(&self, c: Commands) -> String {
        let mut buffer = String::with_capacity(128);
        let conn = Stream::connect(NAME.to_ns_name::<GenericNamespaced>().unwrap()).unwrap();
        let mut conn = BufReader::new(conn);
        let _ = conn.get_mut().write_all(serde_json::to_string(&c).unwrap().as_bytes());
        let _ = conn.get_mut().write_all(b"\n");
        let _ = conn.read_line(&mut buffer);
        //
        // Remove newline from the end
        let mut buffer = buffer.chars();
        buffer.next_back();
        let buffer = buffer.as_str();

        buffer.to_string()
    }

    /// Shutdown the server
    pub fn shutdown(&self) -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Shutdown)).unwrap()
    }
    /// Fetch IDs of remote songs and playlists
    pub fn fetch_artist(&self)                                                 -> Vec<Item>
    {
        serde_json::from_str::<Vec<Item>>(&self.send_command(Commands::FetchArtists)).unwrap()
    }
    /// Fetch IDs of remote songs and playlists
    pub fn fetch_albums(&self)                                                 -> Vec<Item>
    {
        serde_json::from_str::<Vec<Item>>(&self.send_command(Commands::FetchAlbums)).unwrap()
    }
    /// Fetch IDs of remote songs and playlists
    pub fn fetch_playlists(&self)                                                 -> Vec<Item>
    {
        serde_json::from_str::<Vec<Item>>(&self.send_command(Commands::FetchPlaylists)).unwrap()
    }
    /// Fetch IDs of remote songs and playlists
    pub fn fetch_songs(&self)                                                 -> Vec<Item>
    {
        serde_json::from_str::<Vec<Item>>(&self.send_command(Commands::FetchSongs)).unwrap()
    }
    /// Tell the Subsonic server to rescan
    pub fn scan(&self)                                                  -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Scan)).unwrap()
    }
    /// Get the status of playback
    pub fn status(&self)                                                -> Status
    {
        serde_json::from_str::<Status>(&self.send_command(Commands::Status)).unwrap()
    }
    /// Restart currently playing song
    pub fn restart(&self)                                               -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Restart)).unwrap()
    }
    /// Play (unpause) Playback
    pub fn play(&self)                                                  -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Play)).unwrap()
    }
    /// Stop and clear queue
    pub fn stop(&self)                                                  -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Stop)).unwrap()
    }
    /// Pause Playback
    pub fn pause(&self)                                                 -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Pause)).unwrap()
    }
    /// Skip the currentlly playing song
    pub fn skip(&self)                                                  -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Skip)).unwrap()
    }
    /// Add a song to the queue
    pub fn queue_add(&self, id: Item, position: u8)                     -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::QueueAdd{id, position})).unwrap()
    }
    /// Remove a song from the queue
    pub fn queue_remove(&self, id: Item)                                -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::QueueRemove(id))).unwrap()
    }
    /// Adjust volume by percent
    pub fn volume_adjust(&self, amount: u8)                             -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::VolumeAdjust(amount))).unwrap()
    }
    /// Set the volume by percent
    pub fn volume_set(&self, amount: u8)                                -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::VolumeSet(amount))).unwrap()
    }
    /// Search for a query
    pub fn search(&self, query: String)                                 -> Vec<Item>
    {
        serde_json::from_str::<Vec<Item>>(&self.send_command(Commands::Search(query))).unwrap()
    }
    /// Download a song for offline playback
    pub fn download(&self, id: Item)                                    -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Download(id))).unwrap()
    }
    /// Delete a song from offline playback
    pub fn delete(&self, id: Item)                                      -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::Delete(id))).unwrap()
    }
    /// Favorite a song on the Subsonic server
    pub fn star(&self, id: Item)                                        -> bool 
    {
        serde_json::from_str::<bool >(&self.send_command(Commands::Star(id))).unwrap()
    }
    /// Download all the songs from a playlist
    pub fn playlist_download(&self, id: Item)                           -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::PlaylistDownload(id))).unwrap()
    }
    /// Upload changes on a local playlist
    pub fn playlist_upload(&self, id: Item)                             -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::PlaylistUpload(id))).unwrap()
    }
    /// Create a new local playlist
    pub fn playlist_new(&self, name: String)                            -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::PlaylistNew{name})).unwrap()
    }
    /// Add to a local playlist
    pub fn playlist_add_to(&self, playlist: Item, id: Item)             -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::PlaylistAddTo{playlist, id})).unwrap()
    }
    /// Remove from a local playlist
    pub fn playlist_remove_from(&self, playlist: Item, id: Item)        -> bool 
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::PlaylistRemoveFrom{playlist, id})).unwrap()
    }
    /// Delete a local playlist
    pub fn playlist_delete(&self, id: Item)                             -> bool
    {
        serde_json::from_str::<bool>(&self.send_command(Commands::PlaylistDelete(id))).unwrap()
    }
    /// Get the info of a song
    pub fn song_info(&self, id: Item)                                   -> Option<SongInfo>
    {
        serde_json::from_str::<Option<SongInfo>>(&self.send_command(Commands::SongInfo(id))).unwrap()
    }
    /// Get the info of a album
    pub fn album_info(&self, id: Item)                                  -> AlbumInfo
    {
        serde_json::from_str::<AlbumInfo>(&self.send_command(Commands::AlbumInfo(id))).unwrap()
    }
}

#[derive(Deserialize,Serialize, Clone)]
pub struct Status {
    pub playing: bool,
    pub current_song: Option<Item>,
    pub queue: Vec<Item>,
}

#[derive(Deserialize,Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Item {
    pub name: String,
    pub id: String,
    pub image_path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SongInfo {
    pub length: Duration,
    pub album: Item,
    pub artist: String,
}

#[derive(Deserialize,Serialize)]
pub struct AlbumInfo {
    pub songs: Vec<Item>,
    pub artist: String,
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    macro_rules! song_info {
        () => {
            SongInfo { 
                length: core::time::Duration::from_secs(10),
                album: Item
                { 
                    id: String::from("1234"), 
                    image_path: String::from("none"), 
                    name: String::from("Some Album")
                },
                artist: String::from("Some Artist"),
            }
        }
    }

    macro_rules! item {
        () => {
            Item {
                id: String::from("2345"),
                image_path: String::from("none"),
                name: String::from("Some Item"),
            }
        }
    } 

    macro_rules! vec_item {
        () => {
            vec!(item!(), item!(), item!(), item!(), item!(), item!(), item!())
        }
    }

    macro_rules! buffer_test {
        () => {
            "Some really long query that just wont end since I am testing to see if the buffer size is too small or if I need to increase how much the buffer can hold in the ipc portions, Why don't I add more until we reach a size that is larger that lets go ahead and say 255".to_string()
        }
    }

    struct Server;
    impl Daemon for Server 
    {
        fn shutdown(&self) -> bool {
            true
        }

        fn scan(&self)                                                  -> bool {
            true
        }

        fn status(&self)                                                -> &Status {
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
            let _ = (id, position);
            todo!()
        }

        fn queue_remove(&self, id: Item)                                -> bool {
            let _ = id;
            todo!()
        }

        fn volume_adjust(&self, amount: u8)                             -> bool {
            let _ = amount;
            todo!()
        }

        fn volume_set(&self, amount: u8)                                -> bool {
            let _ = amount;
            todo!()
        }

        fn search(&self, query: String)                                 -> Vec<Item> {
            if  query == buffer_test!()
            {
                vec_item!()
            }
            else
            {
                vec!()
            }
        }

        fn download(&self, id: Item)                                    -> bool {
            let _ = id;
            todo!()
        }

        fn delete(&self, id: Item)                                      -> bool {
            let _ = id;
            todo!()
        }

        fn star(&self, id: Item)                                        -> bool {
            let _ = id;
            todo!()
        }

        fn playlist_download(&self, id: Item)                           -> bool {
            let _ = id;
            todo!()
        }

        fn playlist_upload(&self, id: Item)                             -> bool {
            let _ = id;
            todo!()
        }

        fn playlist_new(&self, name: String)                            -> bool {
            let _ = name;
            todo!()
        }

        fn playlist_add_to(&self, playlist: Item, id: Item)             -> bool {
            let _ = (id, playlist);
            todo!()
        }

        fn playlist_remove_from(&self, playlist: Item, id: Item)        -> bool {
            let _ = (id, playlist);
            todo!()
        }

        fn playlist_delete(&self, id: Item)                             -> bool {
            let _ = id;
            todo!()
        }

        fn song_info(&self, id: Item)                                   -> Option<SongInfo> {
            let _ = id;
            Some(song_info!())
        }

        fn album_info(&self, id: Item)                                  -> Option<AlbumInfo> {
            let _ = id;
            todo!()
        }

        fn fetch_artists(&self)                                         -> Vec<Item> {
            todo!()
        }

        fn fetch_albums(&self)                                          -> Vec<Item> {
            todo!()
        }

        fn fetch_playlists(&self)                                       -> Vec<Item> {
            todo!()
        }

        fn fetch_songs(&self)                                           -> Vec<Item> {
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

        assert_eq!(song_info!(), client.song_info(item!()).unwrap());
        assert_eq!(vec_item!(), client.search(buffer_test!()));

        client.shutdown();
    }

}
